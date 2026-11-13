//! Experimental continuous CPU worker over a sparse 512-MiB address window.
//! This is a guest-memory bridge, NOT a commercial Xenon execution environment:
//! imports, exception handling, MMIO, VMX128 and actual Xbox kernel are incomplete.
use crate::{cpu::Cpu,kernel::Kernel,memory::Memory,scheduler::Scheduler};
use jni::{JNIEnv,objects::{JByteArray,JLongArray,JObject},sys::{jbyteArray,jint,jlong,jstring}};
use std::{collections::BTreeMap,fmt::Write,sync::{Mutex,OnceLock,atomic::{AtomicBool,AtomicU64,Ordering}},time::Instant,thread};
const GUEST_BASE:u64=0x8200_0000;
const GUEST_SIZE:usize=512*1024*1024;
const MAX_IMAGE:usize=128*1024*1024;
const UPLOAD_CHUNK:usize=64*1024;
const TEST_ABI:u64=0x5845_0000;

pub struct Session {
    cpu:Cpu, mem:Memory, kernel:Kernel, exec:Vec<(usize,usize)>, scheduler:Scheduler,
    image_base:u64,image_size:usize,loaded:usize,steps:u64,paused:Option<String>,
    last_event:String,last_ips:f64, imports:BTreeMap<usize,(u32,u32)>,
}
impl Session {
    pub fn create_paged(base:u64,entry:u64,size:usize,ranges:&[i64])->Result<Self,String>{
        if size<4||size>MAX_IMAGE {return Err("PE image must fit 4..128MiB".into())}
        let end=base.checked_add(size as u64).ok_or("image address overflow")?;
        if base<GUEST_BASE || end>GUEST_BASE+GUEST_SIZE as u64 || entry<base || entry>=end {
            return Err("PE outside supported virtual RAM window".into());
        }
        if ranges.len()%2!=0||ranges.is_empty()||ranges.len()>256{return Err("invalid PE executable ranges".into())}
        let mut exec=Vec::new();
        for pair in ranges.chunks_exact(2){
            if pair[0]<0||pair[1]<0 {return Err("negative PE range".into());}
            let (low,high)=(pair[0] as u64,pair[1] as u64);
            if low<base||high>end||low>=high{return Err("executable range outside image".into());}
            exec.push((low as usize,high as usize));
        }
        if !exec.iter().any(|&(lo,hi)|entry as usize>=lo && (entry as usize).checked_add(4).is_some_and(|p|p<=hi)){
            return Err("entry point not in executable section".into());
        }
        let mut mem=Memory::sparse(GUEST_BASE,GUEST_SIZE).map_err(str::to_string)?;
        mem.set_page_permissions(base,size,Memory::READ|Memory::WRITE).map_err(str::to_string)?;
        for &(low,high) in &exec {
            mem.set_page_permissions(low as u64,high-low,Memory::READ|Memory::EXECUTE).map_err(str::to_string)?;
        }
        // Temporary test stack. A real Xbox 360 stack comes from kernel thread setup.
        mem.set_page_permissions(GUEST_BASE+GUEST_SIZE as u64-1024*1024,1024*1024,Memory::READ|Memory::WRITE).map_err(str::to_string)?;
        let mut cpu=Cpu::new();cpu.pc=entry as usize;cpu.gpr[1]=GUEST_BASE+GUEST_SIZE as u64-0x1000;
        Ok(Self{cpu,mem,kernel:Kernel::new(),exec,scheduler:Scheduler::new(),image_base:base,
            image_size:size,loaded:0,steps:0,paused:None,last_event:String::new(),last_ips:0.0,
            imports:BTreeMap::new()})
    }
    pub fn create(image:Vec<u8>,base:u64,entry:u64,ranges:&[i64])->Result<Self,String>{
        let mut s=Self::create_paged(base,entry,image.len(),ranges)?;
        for chunk in image.chunks(UPLOAD_CHUNK){s.append(base+s.loaded as u64,chunk)?;}
        Ok(s)
    }
    pub fn append(&mut self,addr:u64,bytes:&[u8])->Result<(),String>{
        if bytes.is_empty()||bytes.len()>UPLOAD_CHUNK||addr!=self.image_base+self.loaded as u64||
            self.loaded.checked_add(bytes.len()).is_none_or(|x|x>self.image_size) {
            return Err("PE chunk out of order, too large or outside image".into());
        }
        self.mem.load_image_bytes(addr as usize,bytes).map_err(str::to_string)?;
        self.loaded+=bytes.len();
        Ok(())
    }
    pub fn set_import_catalog(&mut self,data:&[i64])->Result<usize,String>{
        if data.len()%3!=0 || data.len()>8192*3 {return Err("import catalog invalid length".into());}
        if self.steps>0{return Err("cannot install import catalog after CPU starts".into());}
        let mut imports=BTreeMap::new();
        for chunk in data.chunks_exact(3) {
            let (addr,modid,ordinal)=(chunk[0],chunk[1],chunk[2]);
            if !(0..=u32::MAX as i64).contains(&addr)
                || !(1..=2).contains(&modid) || !(1..=65535).contains(&ordinal) {
                return Err("import thunk invalid address/library/ordinal".into());
            }
            let at=addr as usize;
            if !self.exec.iter().any(|&(lo,hi)|at>=lo && at.checked_add(16).is_some_and(|p|p<=hi)) {
                return Err(format!("import thunk outside executable section 0x{at:08X}"));
            }
            if imports.insert(at,(modid as u32,ordinal as u32)).is_some(){
                return Err(format!("duplicate import thunk at 0x{at:08X}"));
            }
        }
        self.imports=imports;
        Ok(self.imports.len())
    }
    pub fn ready(&self)->bool{self.loaded==self.image_size}
    pub fn read_guest_bytes(&self,address:u64,length:usize)->Result<Vec<u8>,String>{
        if !self.ready() || length==0 || length>16384 || address>u32::MAX as u64 {
            return Err("guest read disabled, uninitialized or too large".into());
        }
        self.mem.read(address as usize,length).map_err(str::to_string)
    }
    pub fn step_chunk(&mut self,limit:usize)->String {
        if !self.ready(){return format!("PAUSED|PE incompleto: {}/{} bytes",self.loaded,self.image_size);}
        if let Some(reason)=&self.paused{return self.report("PAUSED",reason);}
        let start=Instant::now();let mut count=0u64;
        for _ in 0..limit.min(4096){
            let pc=self.cpu.pc;
            if !self.exec.iter().any(|&(lo,hi)|pc>=lo && pc.checked_add(4).is_some_and(|x|x<=hi)){
                self.paused=Some(format!("PC deixou secao executavel: 0x{pc:08X}"));break;
            }
            let opcode=match self.mem.fetch_u32_be(pc){
                Ok(w)=>w,Err(e)=>{self.paused=Some(format!("FALHA EXEC @0x{pc:08X}: {e}"));break;}
            };
            let result=if let Some(&(module,ordinal))=self.imports.get(&pc) {
                let name=if module==1 {"xboxkrnl.exe"}else{"xam.xex"};
                let result=self.kernel.dispatch_xex_thunk(module,ordinal,&mut self.cpu.gpr,&mut self.mem);
                result.and_then(|event| {
                    let return_pc=self.cpu.lr as usize;
                    if !self.exec.iter().any(|&(lo,hi)|return_pc>=lo &&
                         return_pc.checked_add(4).is_some_and(|end|end<=hi)) {
                        return Err(format!("{name} ordinal {ordinal}: LR invalido 0x{return_pc:08X}"));
                    }
                    self.cpu.pc=return_pc;
                    self.last_event=format!("{name} ordinal {ordinal}: {event}");
                    Ok(())
                })
            }else if opcode>>26==17 {
                let service=self.cpu.gpr[0];
                if !(TEST_ABI+1..=TEST_ABI+23).contains(&service){
                    Err(format!("sc sem kernel Xbox HLE: r0=0x{service:X}; import/ordinal nao resolvido"))
                }else{
                    let service_id=(service-TEST_ABI) as u32;
                    let requested_entry=self.cpu.gpr[4] as usize;
                    let res=self.kernel.dispatch_private(service_id,&mut self.cpu.gpr,&mut self.mem);
                    if res.is_ok(){self.cpu.pc=self.cpu.pc.saturating_add(4);}
                    res.and_then(|event|{
                        if service_id==6{
                            if !self.exec.iter().any(|&(lo,hi)|requested_entry>=lo&&requested_entry.checked_add(4).is_some_and(|end|end<=hi)){
                                return Err(format!("ExCreateThread invalid entry 0x{requested_entry:08X}"));
                            }
                            let id=self.scheduler.spawn(requested_entry)?;
                            self.last_event=format!("{event}; diagnostic task {id}");
                        }else{self.last_event=event;}
                        Ok(())
                    })
                }
            }else{self.cpu.step_mut(&mut self.mem)};
            if let Err(e)=result{
                self.paused=Some(format!("PARADA: {e} | PC=0x{pc:08X} OP={opcode:08X}"));break;
            }
            count+=1;self.steps=self.steps.saturating_add(1);
            self.kernel.tick();self.scheduler.tick(&mut self.cpu);
        }
        self.last_ips=count as f64/start.elapsed().as_secs_f64().max(0.000001);
        if self.paused.is_some(){self.report("PAUSED",self.paused.as_deref().unwrap_or(""))}
        else{self.report("RUNNING","")}
    }
    pub fn halt(&mut self){self.paused=Some("Interrompido pelo usuario".into());}
    pub fn snapshot(&self)->String{
        if !self.ready(){return format!("LOADING|PE {}/{} bytes; RAM comprometida={} KiB",self.loaded,self.image_size,self.mem.committed_bytes()/1024)}
        if let Some(reason)=&self.paused{self.report("PAUSED",reason)}else{self.report("RUNNING","")}
    }
    fn report(&self,status:&str,reason:&str)->String{
        let mut s=format!("{status}|IPS={:.0}; INSTRUCOES={}; PC=0x{:08X}; LR=0x{:016X}; CTR=0x{:016X}; RAM_COMMIT={} KiB\n",
            self.last_ips,self.steps,self.cpu.pc,self.cpu.lr,self.cpu.ctr,self.mem.committed_bytes()/1024);
        let _=writeln!(s,"THREAD={} / {} (agendador de teste); IMPORT_THUNKS={}",
            self.scheduler.current_id(),self.scheduler.threads(),self.imports.len());
        let (ring,words,swaps)=self.kernel.gpu_ring_snapshot();
        let _=writeln!(s,"GPU_RING=0x{ring:08X}; GPU_DWORDS={words}; SWAPS={swaps}");
        if !reason.is_empty(){let _=writeln!(s,"{reason}");}
        if !self.last_event.is_empty(){let _=writeln!(s,"HLE: {}",self.last_event);}
        if let Ok(w)=self.mem.read_u32_be(self.cpu.pc){let _=writeln!(s,"Proximo opcode=0x{w:08X}");}
        for reg in 3..=10{let _=writeln!(s,"r{reg}=0x{:016X}",self.cpu.gpr[reg]);}
        s
    }
}
static SESSION:OnceLock<Mutex<Option<Session>>>=OnceLock::new();
static GENERATION:AtomicU64=AtomicU64::new(0);
static ACTIVE:AtomicBool=AtomicBool::new(false);
fn state()->&'static Mutex<Option<Session>>{SESSION.get_or_init(||Mutex::new(None))}
fn output(mut env:JNIEnv,s:String)->jstring{env.new_string(s).map(|x|x.into_raw()).unwrap_or(std::ptr::null_mut())}
fn read_ranges(env:&mut JNIEnv, ranges:JLongArray)->Result<Vec<i64>,String>{
    let n=env.get_array_length(&ranges).map_err(|e|e.to_string())? as usize;
    if n>256{return Err("too many PE ranges".into())}
    let mut r=vec![0i64;n];env.get_long_array_region(&ranges,0,&mut r).map_err(|e|e.to_string())?;Ok(r)
}
fn install(session:Session)->String{
    GENERATION.fetch_add(1,Ordering::SeqCst);ACTIVE.store(false,Ordering::SeqCst);
    match state().lock(){Ok(mut s)=>{*s=Some(session);"OK|Sessao PowerPC preparada".into()},Err(_)=>"ERROR|CPU mutex poisoned".into()}
}
#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeStartSessionPaged(mut env:JNIEnv,_:JObject,
    base:jlong,entry:jlong,image_size:jint,ranges:JLongArray)->jstring{
    let result=(||->Result<Session,String>{
        if base<0||entry<0||image_size<=0{return Err("invalid image metadata".into())}
        let r=read_ranges(&mut env,ranges)?;
        Session::create_paged(base as u64,entry as u64,image_size as usize,&r)
    })();
    output(env,match result{Ok(s)=>install(s),Err(e)=>format!("ERROR|{e}")})
}
#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeUploadSessionChunk(mut env:JNIEnv,_:JObject,
    guest_addr:jlong,data:JByteArray)->jstring{
    let result=(||->Result<String,String>{
        if guest_addr<0||guest_addr>u32::MAX as i64{return Err("invalid upload address".into())}
        let bytes=env.convert_byte_array(data).map_err(|e|e.to_string())?;
        let mut lock=state().lock().map_err(|_|"CPU mutex poisoned")?;
        let session=lock.as_mut().ok_or("session absent")?;
        session.append(guest_addr as u64,&bytes)?;
        Ok(if session.ready(){"OK|PE guest pages complete".into()}else{"OK|PE chunk mapped".into()})
    })();
    output(env,result.unwrap_or_else(|e|format!("ERROR|{e}")))
}
#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeLaunchSession(env:JNIEnv,_:JObject)->jstring{
    let ready=state().lock().ok().and_then(|s|s.as_ref().map(|x|x.ready())).unwrap_or(false);
    if !ready{return output(env,"ERROR|PE pages not fully uploaded".into());}
    if ACTIVE.swap(true,Ordering::SeqCst){return output(env,"ERROR|CPU already running".into());}
    let token=GENERATION.fetch_add(1,Ordering::SeqCst)+1;
    thread::spawn(move ||{
        while ACTIVE.load(Ordering::SeqCst) && GENERATION.load(Ordering::SeqCst)==token{
            let paused=match state().lock(){
                Ok(mut guard)=>guard.as_mut().is_none_or(|s|s.step_chunk(2048).starts_with("PAUSED|")),
                Err(_)=>true,
            };
            if paused {break;}
            thread::sleep(std::time::Duration::from_millis(1));
        }
        if GENERATION.load(Ordering::SeqCst)==token{ACTIVE.store(false,Ordering::SeqCst);}
    });
    output(env,"OK|Thread nativa de CPU iniciada (HLE parcial)".into())
}
#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativePollSession(env:JNIEnv,_:JObject)->jstring{
    let value=match state().lock(){Ok(s)=>s.as_ref().map(|s|s.snapshot()).unwrap_or_else(||"PAUSED|Nenhuma sessao".into()),Err(_)=>"PAUSED|mutex poisoned".into()};output(env,value)
}
// Old JNI is retained for existing test automation and compatibility.
#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeStartSession(mut env:JNIEnv,_:JObject,
    image:JByteArray,base:jlong,entry:jlong,ranges:JLongArray)->jstring{
    let result=(||->Result<Session,String>{
        if base<0||entry<0{return Err("invalid base/entry".into())}
        let bytes=env.convert_byte_array(image).map_err(|e|e.to_string())?;
        let r=read_ranges(&mut env,ranges)?;
        Session::create(bytes,base as u64,entry as u64,&r)
    })();output(env,match result{Ok(s)=>install(s),Err(e)=>format!("ERROR|{e}")})
}
#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeRunSessionChunk(env:JNIEnv,_:JObject,steps:jint)->jstring{
    let msg=match state().lock(){Ok(mut g)=>g.as_mut().map(|s|s.step_chunk(steps.clamp(1,4096) as usize)).unwrap_or_else(||"PAUSED|Nenhuma sessao".into()),Err(_)=>"PAUSED|mutex poisoned".into()};output(env,msg)
}
#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeStopSession(env:JNIEnv,_:JObject)->jstring{
    GENERATION.fetch_add(1,Ordering::SeqCst);ACTIVE.store(false,Ordering::SeqCst);
    let msg=match state().lock(){Ok(mut g)=>{if let Some(s)=g.as_mut(){s.halt();}"PAUSED|CPU interrompida".into()},Err(_)=>"PAUSED|mutex poisoned".into()};output(env,msg)
}
#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeConfigureImportThunks(
    mut env:JNIEnv,_:JObject,triples:JLongArray)->jstring {
    let msg=(||->Result<String,String>{
        let n=env.get_array_length(&triples).map_err(|e|e.to_string())? as usize;
        if n>8192*3 || n%3!=0{return Err("invalid import triple array".into());}
        let mut values=vec![0i64;n];
        env.get_long_array_region(&triples,0,&mut values).map_err(|e|e.to_string())?;
        let mut guard=state().lock().map_err(|_|"session lock poisoned")?;
        let session=guard.as_mut().ok_or("no active session")?;
        let installed=session.set_import_catalog(&values)?;
        Ok(format!("OK|{installed} imported function thunks identified; only TLS ordinals implemented"))
    })();
    output(env,msg.unwrap_or_else(|e|format!("ERROR|{e}")))
}
// PM4 inspection reads only a bounded slice from the active Rust guest RAM.
// No host pointer or large snapshot crosses the JNI boundary.
#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeReadSessionMemory(
    env:JNIEnv,_:JObject,address:jlong,length:jint)->jbyteArray {
    if address<0 || length<=0 || length>16384{return std::ptr::null_mut();}
    let content=state().lock().ok().and_then(|guard|guard.as_ref()
        .and_then(|session|session.read_guest_bytes(address as u64,length as usize).ok()));
    match content.and_then(|bytes|env.byte_array_from_slice(&bytes).ok()) {
        Some(arr)=>arr.into_raw(),None=>std::ptr::null_mut()
    }
}

#[cfg(test)]mod tests{use super::*;
 #[test]fn startup_and_diagnostics(){
  let code=vec![0x38,0x60,0,5,0x38,0x80,0,7,0x7c,0xa3,0x22,0x14,0xff,0xff,0xff,0xff];
  let mut s=Session::create(code,0x82000000,0x82000000,&[0x82000000,0x82000010]).unwrap();
  let msg=s.step_chunk(10);assert!(msg.starts_with("PAUSED|"));assert!(msg.contains("OP=FFFFFFFF"));assert!(msg.contains("r5=0x000000000000000C"));
 }
 #[test]fn paged_streaming_gt16mib(){
  let size=17*1024*1024;let mut s=Session::create_paged(0x82000000,0x82000000,size,&[0x82000000,0x82001000]).unwrap();
  for offset in (0..size).step_by(UPLOAD_CHUNK){let n=(size-offset).min(UPLOAD_CHUNK);s.append(0x82000000+offset as u64,&vec![0;n]).unwrap();}
  assert!(s.ready());assert_eq!(s.mem.committed_bytes(),0);assert_eq!(s.mem.len(),GUEST_SIZE);
 }
 #[test]fn branches_obey_exec(){let code=vec![0x48,0,0,0];let mut s=Session::create(code,0x82000000,0x82000000,&[0x82000000,0x82000004]).unwrap();let msg=s.step_chunk(100);assert!(msg.starts_with("RUNNING|"));}
}

#[cfg(test)] mod xex_import_tests {
    use super::*;
    #[test]
    fn unknown_real_import_pauses_with_ordinal_and_pc(){
        let base=0x82000000_i64;
        let code=vec![0x01,0x00,0x00,0xCC, 0x60,0,0,0, 0x60,0,0,0, 0x60,0,0,0];
        let mut s=Session::create(code,base as u64,base as u64,&[base,base+16]).unwrap();
        assert_eq!(s.set_import_catalog(&[base,1,204]).unwrap(),1);
        let log=s.step_chunk(1);
        assert!(log.starts_with("PAUSED|"),"{log}");
        assert!(log.contains("xboxkrnl.exe ordinal 204"),"{log}");
        assert!(log.contains("PC=0x82000000"),"{log}");
    }
    #[test]
    fn tls_import_dispatches_with_valid_return_address(){
        let base=0x82000000_i64;
        let code=vec![0x01,0,1,0x52, 0x60,0,0,0, 0x60,0,0,0, 0x60,0,0,0];
        let mut s=Session::create(code,base as u64,base as u64,&[base,base+16]).unwrap();
        s.cpu.lr=base as u64+4;
        s.set_import_catalog(&[base,1,338]).unwrap();
        let log=s.step_chunk(1);
        assert!(log.starts_with("RUNNING|"),"{log}");
        assert_eq!(s.cpu.pc,base as usize+4);
    }
    #[test]
    fn import_outside_executable_is_rejected(){
        let base=0x82000000_i64;
        let mut s=Session::create(vec![0x60,0,0,0],base as u64,base as u64,&[base,base+4]).unwrap();
        assert!(s.set_import_catalog(&[base,1,338]).is_err());
    }
}