//! Bounded, pausable PowerPC execution over a guest-addressed PE snapshot.
//! Experimental: imports, full MMU, VMX128, FPU, and Xbox kernel are absent.
use crate::{cpu::Cpu,kernel::Kernel,memory::Memory};
use jni::{JNIEnv,objects::{JByteArray,JLongArray,JObject},sys::{jint,jlong,jstring}};
use std::{fmt::Write,sync::{Mutex,OnceLock},time::Instant};

const MAX_IMAGE:usize=16*1024*1024;
const MAX_STEPS:u64=2_000_000;
const TEST_ABI:u64=0x5845_0000;

pub struct Session {
    cpu:Cpu, mem:Memory, kernel:Kernel, exec:Vec<(usize,usize)>,
    steps:u64, paused:Option<String>, last_event:String,
}
impl Session {
    pub fn create(image:Vec<u8>,base:u64,entry:u64,ranges:&[i64])->Result<Self,String>{
        if image.len()<4 || image.len()>MAX_IMAGE{return Err("PE image must be 4..16MiB maximum snapshot".into())}
        if base>u32::MAX as u64 || entry>u32::MAX as u64 || ranges.len()%2!=0 || ranges.is_empty() || ranges.len()>256{return Err("invalid image or executable ranges".into())}
        let end=base.checked_add(image.len() as u64).ok_or("address overflow")?;
        if end>1u64<<32{return Err("image exceeds 32-bit guest space".into())}
        let mut exec=Vec::new();
        for part in ranges.chunks_exact(2){
            if part[0]<0 || part[1]<0 {return Err("negative executable range".into())}
            let (lo,hi)=(part[0] as u64,part[1] as u64);
            if lo<base || hi>end || lo>=hi{return Err("executable section outside guest image".into())}
            exec.push((lo as usize,hi as usize));
        }
        if !exec.iter().any(|&(lo,hi)|entry as usize>=lo && entry as usize+4<=hi){return Err("entry is not executable".into())}
        let mut cpu=Cpu::new();cpu.pc=entry as usize;
        Ok(Self{cpu,mem:Memory::with_base(image,base),kernel:Kernel::new(),exec,steps:0,paused:None,last_event:String::new()})
    }
    pub fn step_chunk(&mut self,limit:usize)->String{
        if let Some(reason)=&self.paused{return self.report("PAUSED",0.0,reason);}
        let started=Instant::now();let mut executed=0u64;
        for _ in 0..limit.min(10_000){
            if self.steps>=MAX_STEPS {self.paused=Some(format!("Limite educacional de {} instrucoes atingido",MAX_STEPS));break;}
            let pc=self.cpu.pc;
            if !self.exec.iter().any(|&(lo,hi)|pc>=lo && pc.checked_add(4).is_some_and(|x|x<=hi)){
                self.paused=Some(format!("PC deixou secao executavel: 0x{pc:08X}"));break;
            }
            let opcode=match self.mem.read_u32_be(pc){Ok(x)=>x,Err(e)=>{self.paused=Some(format!("FALHA MEM @0x{pc:08X}: {e}"));break;}};
            let result=if opcode>>26==17 {
                let service=self.cpu.gpr[0];
                if !(TEST_ABI+1..=TEST_ABI+10).contains(&service){
                    Err(format!("sc sem ABI de teste registrada r0=0x{service:X}; xboxkrnl/xam nao resolvidos"))
                } else {
                    let res=self.kernel.dispatch_private((service-TEST_ABI) as u32,&mut self.cpu.gpr,&mut self.mem);
                    if res.is_ok(){self.cpu.pc=self.cpu.pc.saturating_add(4);}res.map(|event|{self.last_event=event;})
                }
            }else{self.cpu.step_mut(&mut self.mem)};
            if let Err(e)=result{
                self.paused=Some(format!("PARADA: {e} | PC=0x{pc:08X} OP={opcode:08X}"));break;
            }
            self.kernel.tick();self.steps+=1;executed+=1;
        }
        let sec=started.elapsed().as_secs_f64().max(1e-6);
        let ips=executed as f64/sec;
        let status=if self.paused.is_some(){"PAUSED"}else{"RUNNING"};
        self.report(status,ips,self.paused.as_deref().unwrap_or(""))
    }
    pub fn halt(&mut self){self.paused=Some("interrompido pelo usuario".into());}
    fn report(&self,status:&str,ips:f64,reason:&str)->String{
        let mut s=format!("{status}|IPS={ips:.0}; INSTRUCOES={}; PC=0x{:08X}; LR=0x{:016X}; CTR=0x{:016X}; CR=0x{:08X}; RAM={} MiB\n",
            self.steps,self.cpu.pc,self.cpu.lr,self.cpu.ctr,self.cpu.cr,self.mem.len()/1024/1024);
        if !reason.is_empty(){let _=writeln!(s,"{reason}");}
        if !self.last_event.is_empty(){let _=writeln!(s,"HLE: {}",self.last_event);}
        let opcode=self.mem.read_u32_be(self.cpu.pc).ok();
        if let Some(word)=opcode {let _=writeln!(s,"Proximo opcode=0x{word:08X}");}
        for reg in 3..=10{let _=writeln!(s,"r{reg}=0x{:016X}",self.cpu.gpr[reg]);}
        s
    }
}
static SESSION:OnceLock<Mutex<Option<Session>>>=OnceLock::new();
fn state()->&'static Mutex<Option<Session>>{SESSION.get_or_init(||Mutex::new(None))}
fn jstring_result(mut env:JNIEnv,s:String)->jstring{env.new_string(s).map(|x|x.into_raw()).unwrap_or(std::ptr::null_mut())}
#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeStartSession(mut env:JNIEnv,_:JObject, image:JByteArray,base:jlong,entry:jlong,ranges:JLongArray)->jstring{
    let res=(||->Result<Session,String>{
        let bytes=env.convert_byte_array(&image).map_err(|e|e.to_string())?;
        let count=env.get_array_length(&ranges).map_err(|e|e.to_string())? as usize;
        if count>256{return Err("too many section ranges".into())}
        let mut nums=vec![0_i64;count];env.get_long_array_region(&ranges,0,&mut nums).map_err(|e|e.to_string())?;
        if base<0||entry<0{return Err("negative base or entry".into())}
        Session::create(bytes,base as u64,entry as u64,&nums)
    })();
    let msg=match res {
        Ok(session)=> match state().lock(){Ok(mut guard)=>{*guard=Some(session);"OK|Sessao PowerPC criada (trace limitado; HLE sinteticamente mapeado)".to_owned()},Err(_)=>"ERROR|mutex poisoned".into()},
        Err(e)=>format!("ERROR|{e}"),
    };jstring_result(env,msg)
}
#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeRunSessionChunk(env:JNIEnv,_:JObject,steps:jint)->jstring{
    let msg=match state().lock(){Ok(mut g)=>match g.as_mut(){Some(s)=>s.step_chunk(steps.clamp(1,10000) as usize),None=>"PAUSED|Nenhuma sessao iniciada".into()},Err(_)=>"PAUSED|mutex poisoned".into()};
    jstring_result(env,msg)
}
#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeStopSession(env:JNIEnv,_:JObject)->jstring{
    let msg=match state().lock(){Ok(mut g)=>{if let Some(s)=g.as_mut(){s.halt();} "PAUSED|Sessao interrompida".into()},Err(_)=>"PAUSED|mutex poisoned".into()};jstring_result(env,msg)
}
#[cfg(test)]mod tests {
 use super::*;
 #[test]fn detects_unsupported_and_preserves_regs(){
  let code=vec![0x38,0x60,0,5,0x38,0x80,0,7,0x7c,0xa3,0x22,0x14,0xff,0xff,0xff,0xff];
  let mut s=Session::create(code,0x82000000,0x82000000,&[0x82000000,0x82000010]).unwrap();
  let msg=s.step_chunk(10);assert!(msg.starts_with("PAUSED|"));assert!(msg.contains("OP=FFFFFFFF"));assert!(msg.contains("r5=0x000000000000000C"));
 }
 #[test]fn branches_obey_exec(){let code=vec![0x48,0,0,0];let mut s=Session::create(code,0x82000000,0x82000000,&[0x82000000,0x82000004]).unwrap();let msg=s.step_chunk(100);assert!(msg.starts_with("RUNNING|"));}
}