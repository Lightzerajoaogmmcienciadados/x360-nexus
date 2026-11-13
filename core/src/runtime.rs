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
const MAIN_STACK_BOTTOM:u64=0x7001_0000; // guard: 0x70000000..0x7000FFFF
const MAIN_STACK_TOP:u64=0x7010_0000;
const MAIN_PCR:u64=0x7100_0000;
const MAIN_THREAD:u64=0x7100_1000;
const MAIN_TLS:u64=0x7101_0000;
// Validated type-0 import slots receive owned backing cells here; these
// are placeholders for exports not yet implemented, not real Xboxkrnl state.
const EXPORT_VARIABLE_BASE:u64=0x7200_0000;
const EXPORT_VARIABLE_STRIDE:u64=256;
const MAX_IMPORT_VARIABLES:usize=8192;
const MOCK_MODULE_BASE:u64=0x7400_0100;
const MOCK_XEX_HEADER:u64=0x7400_0200;

/// Limited user-mode bootstrap model based on known Xbox 360 KPCR offsets.
/// It is not the complete kernel KTHREAD/TEB initialization sequence.
fn setup_main_thread(mem:&mut Memory,cpu:&mut Cpu)->Result<(),String>{
    const RW:u8=Memory::READ|Memory::WRITE;
    mem.set_page_permissions(MAIN_STACK_BOTTOM,(MAIN_STACK_TOP-MAIN_STACK_BOTTOM) as usize,RW)
        .map_err(str::to_string)?;
    mem.set_page_permissions(MAIN_PCR,0x3000,RW).map_err(str::to_string)?;
    mem.set_page_permissions(MAIN_TLS,0x4000,RW).map_err(str::to_string)?;
    // X_KPCR fields are guest big endian.
    mem.write_u32_be(MAIN_PCR as usize,MAIN_TLS as u32).map_err(str::to_string)?; // +0 TLS
    mem.write_u32_be((MAIN_PCR+0x30) as usize,MAIN_PCR as u32).map_err(str::to_string)?;
    mem.write_u32_be((MAIN_PCR+0x70) as usize,MAIN_STACK_TOP as u32).map_err(str::to_string)?;
    mem.write_u32_be((MAIN_PCR+0x74) as usize,MAIN_STACK_BOTTOM as u32).map_err(str::to_string)?;
    mem.write_u32_be((MAIN_PCR+0x100) as usize,MAIN_THREAD as u32).map_err(str::to_string)?;
    // Minimal guest KTHREAD metadata, not a commercial KTHREAD ABI.
    mem.write_u32_be((MAIN_THREAD+0x5C) as usize,MAIN_STACK_TOP as u32).map_err(str::to_string)?;
    mem.write_u32_be((MAIN_THREAD+0x60) as usize,MAIN_STACK_BOTTOM as u32).map_err(str::to_string)?;
    mem.write_u32_be((MAIN_THREAD+0x68) as usize,MAIN_TLS as u32).map_err(str::to_string)?;
    mem.write_u32_be((MAIN_THREAD+0x14C) as usize,1).map_err(str::to_string)?;
    cpu.gpr[1]=MAIN_STACK_TOP-0x100; // 16-byte aligned, guard below stack
    cpu.gpr[13]=MAIN_PCR;
    cpu.gpr[3]=0;cpu.gpr[4]=0;cpu.gpr[5]=0;cpu.gpr[6]=0;
    Ok(())
}

fn setup_additional_thread(mem:&mut Memory,cpu:&mut Cpu,id:u32)->Result<(),String>{
    if !(2..=8).contains(&id){return Err("thread count exhausted".into())}
    let base=0x7200_0000u64 + (id as u64)*0x40000;
    let pcr=0x7300_0000u64+(id as u64)*0x10000;
    let tls=pcr+0x3000;
    let thread=pcr+0x1000;
    const RW:u8=Memory::READ|Memory::WRITE;
    mem.set_page_permissions(base+0x1000,0x3F000,RW).map_err(str::to_string)?;
    mem.set_page_permissions(pcr,0x4000,RW).map_err(str::to_string)?;
    for (off,value) in [(0,tls as u32),(0x30,pcr as u32),(0x70,(base+0x40000) as u32),
                        (0x74,(base+0x1000) as u32),(0x100,thread as u32)] {
        mem.write_u32_be((pcr+off) as usize,value).map_err(str::to_string)?;
    }
    mem.write_u32_be((thread+0x14C) as usize,id).map_err(str::to_string)?;
    cpu.gpr[1]=base+0x40000-0x100;cpu.gpr[13]=pcr;
    Ok(())
}

fn fault_instruction(cpu:&Cpu,word:u32)->String {
    let op=word>>26;let rt=(word>>21)&31;let ra=((word>>16)&31) as usize;
    let rb=((word>>11)&31) as usize;
    let disp=word as u16 as i16 as i64;
    match op {
        14|15=>return format!("DISASM={} r{rt},r{ra},{disp}",if op==14{"addi"}else{"addis"}),
        24|25=>return format!("DISASM={} r{ra},r{rt},0x{:04X}",
            if op==24{"ori"}else{"oris"},word&0xFFFF),
        18=>return format!("DISASM=b{} 0x{:08X}",if word&1!=0{"l"}else{""},
            (cpu.pc as u64).wrapping_add(((((word&0x03FF_FFFC) as i32)<<6)>>6) as i64 as u64)),
        16=>{
            let bo=(word>>21)&31;let bi=(word>>16)&31;
            let cr_bit=((cpu.cr>>(31-bi))&1)!=0;
            let ctr_after=if bo&4!=0{cpu.ctr}else{cpu.ctr.wrapping_sub(1)};
            let cond=bo&16!=0 || cr_bit==(bo&8!=0);
            let counter=bo&4!=0 || ((ctr_after!=0)!=(bo&2!=0));
            let target=if word&2!=0{(word as u16 as i16 as i64) as u64}
                else{(cpu.pc as u64).wrapping_add(word as u16 as i16 as i64 as u64)};
            return format!("DISASM=bc BO={bo} BI={bi} CRbit={} CTR=0x{:X} taken={} target=0x{target:08X}",
                u8::from(cr_bit),cpu.ctr,cond&&counter);
        },
        19=>{
            let xo=(word>>1)&1023;
            if xo==16||xo==528 {return format!("DISASM={} BO={} BI={} LR=0x{:X} CTR=0x{:X}",
                if xo==16{"bclr"}else{"bcctr"},(word>>21)&31,(word>>16)&31,cpu.lr,cpu.ctr);}
        },
        17=>return "DISASM=sc (syscall)".into(),
        _=>(),
    }
    let dname=match op {
        32=>"lwz",33=>"lwzu",34=>"lbz",36=>"stw",37=>"stwu",38=>"stb",
        40=>"lhz",44=>"sth",48=>"lfs",50=>"lfd",52=>"stfs",54=>"stfd",_=>"",
    };
    if !dname.is_empty(){
        let base=if ra==0 {0} else {cpu.gpr[ra]};
        let ea=base.wrapping_add(disp as u64);
        return format!("DISASM={dname} r{rt},{disp}(r{ra}) EA=0x{ea:08X} BASE=r{ra}=0x{base:016X}");
    }
    if op==58||op==62{
        let base=if ra==0{0}else{cpu.gpr[ra]};
        let d=((word&0xFFFC) as i16) as i64;
        let ea=base.wrapping_add(d as u64);
        return format!("DISASM={} r{rt},{d}(r{ra}) EA=0x{ea:08X}",if op==58{"ld"}else{"std"});
    }
    if op==31 {
        let xo=(word>>1)&1023;
        if matches!(xo,23|151|87|215|279|407) {
            let ea=cpu.gpr[ra].wrapping_add(cpu.gpr[rb]);
            return format!("DISASM=X-form XO={xo} r{rt},r{ra},r{rb} EA=0x{ea:08X}");
        }
        return format!("DISASM=extended PowerPC XO={xo}");
    }
    format!("DISASM=PowerPC primary opcode={op}")
}

pub struct Session {
    cpu:Cpu, mem:Memory, kernel:Kernel, exec:Vec<(usize,usize)>, scheduler:Scheduler,
    image_base:u64,image_size:usize,loaded:usize,steps:u64,paused:Option<String>,
    last_event:String,last_ips:f64, imports:BTreeMap<usize,(u32,u32)>,
    variables:BTreeMap<usize,(u32,u32,u32)>,
    first_instruction:Option<String>, known_variables:usize, boot_clock:Instant,
    boot_compatibility:Option<String>,
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
        let mut cpu=Cpu::new();cpu.pc=entry as usize;
        setup_main_thread(&mut mem,&mut cpu)?;
        let mut kernel=Kernel::new();
        kernel.set_guest_heap_base(0x2000_0000)?;
        Ok(Self{cpu,mem,kernel,exec,scheduler:Scheduler::new(),image_base:base,
            image_size:size,loaded:0,steps:0,paused:None,last_event:String::new(),last_ips:0.0,
            imports:BTreeMap::new(),variables:BTreeMap::new(),
            first_instruction:None,known_variables:0,boot_clock:Instant::now(),
            boot_compatibility:None})
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
    /// Install only type-0 records that still contain the original XEX
    /// descriptor. A separate low VA cell is provided, but unknown exports
    /// remain empty: non-zero pointer != implemented kernel variable.
    pub fn set_variable_catalog(&mut self,data:&[i64])->Result<usize,String>{
        if !self.ready() || self.steps>0{return Err("cannot configure variables after CPU launch or before PE upload".into())}
        if data.len()%3!=0 || data.len()/3>MAX_IMPORT_VARIABLES{
            return Err("invalid variable triples".into());
        }
        let mut staged=BTreeMap::new();
        for chunk in data.chunks_exact(3){
            let (addr,module,ordinal)=(chunk[0],chunk[1],chunk[2]);
            if !(0..=u32::MAX as i64).contains(&addr)
                || !(1..=2).contains(&module) || !(1..=65535).contains(&ordinal){
                return Err("invalid variable slot/module/ordinal".into());
            }
            let at=addr as usize;
            let end=self.image_base.checked_add(self.image_size as u64).ok_or("image overflow")?;
            if (at as u64)<self.image_base || (at as u64).checked_add(4).is_none_or(|v|v>end)
               {
                return Err(format!("variable import slot outside image 0x{at:08X}"));
            }
            let value=self.mem.read_u32_be(at).map_err(str::to_string)?;
            if value>>24!=0 || value&0xffff!=ordinal as u32 {
                return Err(format!("unexpected variable descriptor @0x{at:08X} value=0x{value:08X}"));
            }
            if staged.insert(at,(module as u32,ordinal as u32)).is_some(){
                return Err(format!("duplicate variable slot 0x{at:08X}"));
            }
        }
        // Keep the v5.4 secondary-thread stack region (>0x72080000) free.
        if staged.len() as u64*EXPORT_VARIABLE_STRIDE > 0x80000 {
            return Err("import variable backing capacity exceeded".into());
        }
        if staged.is_empty(){self.variables.clear();return Ok(0);}
        let region_len=(staged.len() as u64*EXPORT_VARIABLE_STRIDE+4095)&!4095;
        self.mem.set_page_permissions(EXPORT_VARIABLE_BASE,region_len as usize,
            Memory::READ|Memory::WRITE).map_err(str::to_string)?;
        let mut variables=BTreeMap::new();
        let mut known_variables=0usize;
        // Small guest-side loader object for a *test* executable handle.
        // This is not the complete XEX header/module loader ABI.
        if staged.values().any(|&(m,o)|m==1 && o==403) {
            self.mem.set_page_permissions(0x7400_0000,4096,Memory::READ|Memory::WRITE)
                .map_err(str::to_string)?;
            self.mem.write_u32_be((MOCK_MODULE_BASE+0x58) as usize,MOCK_XEX_HEADER as u32)
                .map_err(str::to_string)?;
            self.mem.write(MOCK_XEX_HEADER as usize,b"XEX2").map_err(str::to_string)?;
        }
        for (i,(&slot,&(module,ordinal))) in staged.iter().enumerate(){
            let pointer=EXPORT_VARIABLE_BASE+i as u64*EXPORT_VARIABLE_STRIDE;
            // An IAT type-0 import refers to an exported variable. The slot
            // receives a guest pointer; only a small, explicitly understood
            // subset gets guest-side data. Other cells remain zeroed and
            // unresolved, not fake working kernel exports.
            if module==1 {
                let mut recognized=true;
                match ordinal {
                    342=>{ // XboxHardwareInfo, flags + CPU count (synthetic)
                        self.mem.write_u32_be(pointer as usize,0x20).map_err(str::to_string)?;
                        self.mem.write_u8((pointer+4) as usize,6).map_err(str::to_string)?;
                    },
                    344=>{ // XboxKrnlVersion (BE16 2.0.17559.0)
                        for (j,n) in [2u16,0,17559,0].iter().enumerate(){
                            self.mem.write_u16_be((pointer+2*j as u64) as usize,*n)
                                .map_err(str::to_string)?;
                        }
                    },
                    173=>{ // KeTimeStampBundle: initial zeroed timestamp
                        self.mem.write_u32_be((pointer+16) as usize,0)
                            .map_err(str::to_string)?;
                    },
                    403=>{ // XexExecutableModuleHandle: simplified loader entry
                        self.mem.write_u32_be(pointer as usize,MOCK_MODULE_BASE as u32)
                            .map_err(str::to_string)?;
                    },
                    430=>{self.mem.write(pointer as usize,b"\"default.xex\"\0")
                        .map_err(str::to_string)?;},
                    431=>{self.mem.write(pointer as usize,b"game:\\default.xex\0")
                        .map_err(str::to_string)?;},
                    446|447=>{ // Bounded synthetic device object, not a working Xenos driver.
                        let object=pointer+0x40;
                        self.mem.write_u32_be(pointer as usize,object as u32).map_err(str::to_string)?;
                        self.mem.write_u32_be(object as usize,object as u32).map_err(str::to_string)?;
                    },
                    448=>{self.mem.write_u32_be(pointer as usize,500).map_err(str::to_string)?;},
                    _=>recognized=false,
                }
                if recognized {known_variables+=1;}
            }
            self.mem.load_image_bytes(slot,&(pointer as u32).to_be_bytes())
                .map_err(str::to_string)?;
            variables.insert(slot,(module,ordinal,pointer as u32));
        }
        self.variables=variables;
        self.known_variables=known_variables;
        Ok(self.variables.len())
    }
    /// Opt-in, title- and code-signature-guarded experiment for the
    /// user's observed Minecraft entrypoint. This is NOT a documented
    /// Xbox 360 OS invariant: all other titles retain architectural CR=0.
    pub fn configure_boot_cr(&mut self,title_id:u32)->Result<String,String>{
        if !self.ready() || self.steps!=0{return Err("boot CR requires uploaded image before first instruction".into());}
        self.boot_compatibility=None;
        if title_id!=0x4D53_0A81 {return Ok("OK|CR architecturally zero; no title-specific compatibility".into());}
        if self.cpu.pc!=0x8214_AB10{return Ok(format!(
            "OK|Minecraft title ID present but PC=0x{:08X} differs; CR unchanged",self.cpu.pc));}
        let op=self.mem.fetch_u32_be(self.cpu.pc).map_err(str::to_string)?;
        if op!=0x419A_0088{return Ok(format!(
            "OK|Minecraft entry opcode 0x{op:08X} differs; CR unchanged"));}
        // 0x419A0088 is bc BO=12 BI=26 +0x88: CR6.EQ must be set to take it.
        self.cpu.cr|=1u32<<(31-26);
        self.boot_compatibility=Some("Minecraft 0x4D530A81 exact-entry override: CR6.EQ=1 (hypothesis; NOT Xbox kernel ABI)".into());
        Ok(format!("OK|{}; CR=0x{:08X}",self.boot_compatibility.as_deref().unwrap_or(""),self.cpu.cr))
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
            let first=self.steps==0 && self.first_instruction.is_none();
            if first {
                self.first_instruction=Some(format!(
                    "INSTRUCAO #0: PC=0x{pc:08X} OP={opcode:08X} | {} | ANTES r1=0x{:08X} r3=0x{:08X} r13=0x{:08X} r30=0x{:08X} r31=0x{:08X}",
                    fault_instruction(&self.cpu,opcode), self.cpu.gpr[1],self.cpu.gpr[3],
                    self.cpu.gpr[13],self.cpu.gpr[30],self.cpu.gpr[31]));
            }
            let result=if let Some(&(module,ordinal))=self.imports.get(&pc) {
                let name=if module==1 {"xboxkrnl.exe"}else{"xam.xex"};
                // Real ExCreateThread takes guest entry at r7 and context at r8.
                let thread_entry=self.cpu.gpr[7] as usize;
                let thread_context=self.cpu.gpr[8];
                let result=self.kernel.dispatch_xex_thunk(module,ordinal,&mut self.cpu.gpr,&mut self.mem);
                result.and_then(|event| {
                    let return_pc=self.cpu.lr as usize;
                    if !self.exec.iter().any(|&(lo,hi)|return_pc>=lo &&
                         return_pc.checked_add(4).is_some_and(|end|end<=hi)) {
                        return Err(format!("{name} ordinal {ordinal}: LR invalido 0x{return_pc:08X}"));
                    }
                    self.cpu.pc=return_pc;
                    self.last_event=format!("{name} ordinal {ordinal}: {event}");
                    if module==1 && ordinal==603 {
                        let writes=self.mem.take_gpu_ring_writes();
                        self.kernel.capture_gpu_ring_write(writes);
                        if let Some((address,words))=writes {
                            self.last_event=format!("{event}; PM4 guest write 0x{address:08X} ({words} dwords)");
                        }
                    }
                    if module==1 && ordinal==13 {
                        if !self.exec.iter().any(|&(lo,hi)|thread_entry>=lo &&
                              thread_entry.checked_add(4).is_some_and(|end|end<=hi)) {
                            return Err(format!("ExCreateThread: invalid entry 0x{thread_entry:08X}"));
                        }
                        let mut child=Cpu::new();child.pc=thread_entry;child.gpr[3]=thread_context;
                        let future_id=self.scheduler.next_guest_id();
                        setup_additional_thread(&mut self.mem,&mut child,future_id)?;
                        let id=self.scheduler.spawn_cpu(child)?;
                        self.last_event=format!("{event}; guest thread {id} scheduled");
                    }
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
                            let mut thread_cpu=Cpu::new();thread_cpu.pc=requested_entry;
                            // Prepare a separate stack and PCR before scheduling.
                            let future_id=self.scheduler.next_guest_id();
                            setup_additional_thread(&mut self.mem,&mut thread_cpu,future_id)?;
                            let id=self.scheduler.spawn_cpu(thread_cpu)?;
                            self.last_event=format!("{event}; cooperative guest thread {id}");
                        }else{self.last_event=event;}
                        Ok(())
                    })
                }
            }else{self.cpu.step_mut(&mut self.mem)};
            if let Err(e)=result{
                self.paused=Some(format!("PARADA: {e} | PC=0x{pc:08X} OP={opcode:08X} | {}",fault_instruction(&self.cpu,opcode)));break;
            }
            if first {
                if let Some(msg)=&mut self.first_instruction {
                    let _=write!(msg," | DEPOIS r1=0x{:08X} r3=0x{:08X} r13=0x{:08X} r30=0x{:08X} r31=0x{:08X} NEXT_PC=0x{:08X}",
                        self.cpu.gpr[1],self.cpu.gpr[3],self.cpu.gpr[13],
                        self.cpu.gpr[30],self.cpu.gpr[31],self.cpu.pc);
                }
            }
            count+=1;self.steps=self.steps.saturating_add(1);
            self.kernel.tick();self.scheduler.tick(&mut self.cpu);
        }
        // 1-ms-resolution guest uptime approximation for a supported
        // KeTimeStampBundle import (ordinal 173). Never mutate foreign slots.
        for (_, &(module,ordinal,cell)) in &self.variables {
            if module==1 && ordinal==173 {
                let ms=self.boot_clock.elapsed().as_millis().min(u32::MAX as u128) as u32;
                let _=self.mem.write_u32_be(cell as usize+16,ms);
            }
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
        let _=writeln!(s,"IMPORT_VARIABLE_CELLS={} EXPORTS_INICIALIZADOS={} (modelo parcial; outros nao implementados)",self.variables.len(),self.known_variables);
        if let Some(compat)=&self.boot_compatibility{let _=writeln!(s,"BOOT_COMPAT={compat}");}
        let _=writeln!(s,"CR=0x{:08X}; CR6_EQ={}",self.cpu.cr,(self.cpu.cr>>5)&1);
        if let Some(first)=&self.first_instruction{let _=writeln!(s,"{first}");}
        let (ring,words,swaps)=self.kernel.gpu_ring_snapshot();
        let _=writeln!(s,"GPU_RING=0x{ring:08X}; GPU_DWORDS={words}; SWAPS={swaps}");
        if !reason.is_empty(){let _=writeln!(s,"{reason}");}
        if !self.last_event.is_empty(){let _=writeln!(s,"HLE: {}",self.last_event);}
        if let Ok(w)=self.mem.read_u32_be(self.cpu.pc){let _=writeln!(s,"Proximo opcode=0x{w:08X}");}
        let _=writeln!(s,"CONTEXT: SP=r1=0x{:016X}; KPCR=r13=0x{:016X}; TLS=0x{MAIN_TLS:08X}",self.cpu.gpr[1],self.cpu.gpr[13]);
        for reg in [1,3,4,5,6,10,11,13,30,31] {
            let _=writeln!(s,"r{reg}=0x{:016X}",self.cpu.gpr[reg]);
        }
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
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeConfigureBootCr(env:JNIEnv,_:JObject,title_id:jlong)->jstring{
    let result=if !(0..=u32::MAX as i64).contains(&title_id){
        Err("invalid title ID".into())
    }else{
        state().lock().map_err(|_|"CPU session mutex poisoned".to_string())
            .and_then(|mut lock|lock.as_mut().ok_or("session not started".to_string())?
                .configure_boot_cr(title_id as u32))
    };
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
        Ok(format!("OK|{installed} imported function thunks registered for verified HLE dispatch"))
    })();
    output(env,msg.unwrap_or_else(|e|format!("ERROR|{e}")))
}
#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeConfigureImportVariables(
    mut env:JNIEnv,_:JObject,triples:JLongArray)->jstring {
    let msg=(||->Result<String,String>{
        let n=env.get_array_length(&triples).map_err(|e|e.to_string())? as usize;
        if n>MAX_IMPORT_VARIABLES*3 || n%3!=0{return Err("invalid import variable count".into());}
        let mut values=vec![0i64;n];
        env.get_long_array_region(&triples,0,&mut values).map_err(|e|e.to_string())?;
        let mut state=state().lock().map_err(|_|"session lock poisoned")?;
        let session=state.as_mut().ok_or("no guest session")?;
        let installed=session.set_variable_catalog(&values)?;
        Ok(format!("OK|{installed} validated import variable slots with guest backing (unknown exports not HLE implemented)"))
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
  // The main-thread KPCR and TLS bootstrap intentionally commit two 4-KiB pages.
  assert!(s.ready());assert_eq!(s.mem.committed_bytes(),8192);assert_eq!(s.mem.len(),GUEST_SIZE);
 }
 #[test]fn branches_obey_exec(){let code=vec![0x48,0,0,0];let mut s=Session::create(code,0x82000000,0x82000000,&[0x82000000,0x82000004]).unwrap();let msg=s.step_chunk(100);assert!(msg.starts_with("RUNNING|"));}
}

#[cfg(test)] mod xex_import_tests {
    use super::*;
    #[test]
    fn unknown_real_import_pauses_with_ordinal_and_pc(){
        let base=0x82000000_i64;
        let code=vec![0x01,0x00,0x03,0xE7, 0x60,0,0,0, 0x60,0,0,0, 0x60,0,0,0];
        let mut s=Session::create(code,base as u64,base as u64,&[base,base+16]).unwrap();
        assert_eq!(s.set_import_catalog(&[base,1,999]).unwrap(),1);
        let log=s.step_chunk(1);
        assert!(log.starts_with("PAUSED|"),"{log}");
        assert!(log.contains("xboxkrnl.exe ordinal 999"),"{log}");
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

#[cfg(test)] mod xbox_thread_boot_tests {
    use super::*;
    #[test] fn creates_stack_pcr_and_tls_before_guest_execution(){
        let s=Session::create(vec![0x60,0,0,0],0x82000000,0x82000000,&[0x82000000,0x82000004]).unwrap();
        assert_eq!(s.cpu.gpr[1]&15,0);
        assert_eq!(s.cpu.gpr[13],MAIN_PCR);
        assert_eq!(s.mem.read_u32_be(MAIN_PCR as usize).unwrap(),MAIN_TLS as u32);
        assert_eq!(s.mem.read_u32_be((MAIN_PCR+0x100) as usize).unwrap(),MAIN_THREAD as u32);
        assert!(s.mem.read_u32_be(0x70000000).is_err());
    }
    #[test] fn missing_base_register_reports_effective_address(){
        // nop; lwz r11,0x94(r30) -- intentionally zero r30.
        let code=[0x60,0,0,0,0x81,0x7E,0,0x94];
        let mut s=Session::create(code.to_vec(),0x82000000,0x82000000,&[0x82000000,0x82000008]).unwrap();
        let report=s.step_chunk(2);
        assert!(report.contains("EA=0x00000094"),"{report}");
        assert!(report.contains("r30=0x0000000000000000"),"{report}");
        assert!(report.contains("PARADA:"),"{report}");
    }
}

#[cfg(test)] mod import_variable_tests {
    use super::*;
    #[test]fn maps_variable_backing_without_clobbering_code(){
        let base=0x82000000u64;
        let mut code=vec![0u8;0x2000];
        code[0..4].copy_from_slice(&[0x60,0,0,0]);
        code[0x1100..0x1104].copy_from_slice(&0x0000_0077u32.to_be_bytes());
        let mut s=Session::create(code,base,base,&[base as i64,base as i64+0x1000]).unwrap();
        assert_eq!(s.set_variable_catalog(&[(base+0x1100) as i64,1,119]).unwrap(),1);
        assert_eq!(s.mem.read_u32_be((base+0x1100) as usize).unwrap(),EXPORT_VARIABLE_BASE as u32);
        assert_eq!(s.mem.read_u32_be(EXPORT_VARIABLE_BASE as usize).unwrap(),0);
        assert_eq!(s.mem.fetch_u32_be(base as usize).unwrap(),0x60000000);
    }
    #[test]fn rejects_unknown_descriptor_and_executable_slot(){
        let base=0x82000000u64;
        let mut code=vec![0u8;0x2000];
        code[0x1100..0x1104].copy_from_slice(&0x48000010u32.to_be_bytes());
        let mut s=Session::create(code,base,base,&[base as i64,base as i64+0x1000]).unwrap();
        assert!(s.set_variable_catalog(&[(base+0x1100) as i64,1,0x10]).is_err());
        assert!(s.set_variable_catalog(&[base as i64,1,15]).is_err());
    }
}

#[cfg(test)] mod v56_boot_regression {
    use super::*;
    #[test] fn first_instruction_shows_register_transition() {
        let base=0x82000000u64;
        // addi r30,r0,7; invalid opcode after first instruction
        let mut session=Session::create(vec![0x3B,0xC0,0,7,0xFF,0xFF,0xFF,0xFF],
            base,base,&[base as i64,base as i64+8]).unwrap();
        let report=session.step_chunk(2);
        assert!(report.contains("INSTRUCAO #0: PC=0x82000000"),"{report}");
        assert!(report.contains("DEPOIS r1="),"{report}");
        assert!(report.contains("r30=0x00000007"),"{report}");
        assert!(report.contains("PC=0x82000004"),"{report}");
    }
    #[test] fn recognized_export_variables_are_initialized_by_real_ordinals() {
        let base=0x82000000u64;
        let mut s=Session::create(vec![0x60,0,0,0,0,0x00,0x01,0x56,0,0x00,0x01,0xC0],
            base,base,&[base as i64,base as i64+4]).unwrap();
        assert_eq!(s.set_variable_catalog(&[base as i64+4,1,342,base as i64+8,1,448]).unwrap(),2);
        assert_eq!(s.known_variables,2);
        let info=s.mem.read_u32_be((EXPORT_VARIABLE_BASE) as usize).unwrap();
        assert_eq!(info,0x20);
        let clock=s.mem.read_u32_be((EXPORT_VARIABLE_BASE+EXPORT_VARIABLE_STRIDE) as usize).unwrap();
        assert_eq!(clock,500);
    }
}
#[cfg(test)] mod v56_hle_input_and_first_pc {
    use super::*;
    #[test] fn xam_reads_virtual_gamepad_state(){
        let mut kernel=Kernel::new();let mut mem=Memory::new(4096);let mut gpr=[0u64;32];
        gpr[3]=0;gpr[5]=0x100;
        assert!(kernel.dispatch_xex_thunk(2,0x191,&mut gpr,&mut mem).is_ok());
        assert_eq!(gpr[3],0);
        assert_eq!(mem.read_u32_be(0x100).unwrap(),0);
        assert_eq!(mem.read_u32_be(0x104).unwrap(),0);
        assert_eq!(kernel.dispatch_xex_thunk(2,0x210,&mut gpr,&mut mem).is_ok(),true);
        assert_eq!(gpr[3],0);
    }
    #[test] fn minecraft_entry_diagnostic_keeps_exact_first_pc(){
        let pc=0x8214AB10u64;
        let mut session=Session::create(vec![0x60,0,0,0,0x81,0x7E,0,0x94],pc,pc,&[pc as i64,pc as i64+8]).unwrap();
        let log=session.step_chunk(2);
        assert!(log.contains("INSTRUCAO #0: PC=0x8214AB10"),"{log}");
        assert!(log.contains("EA=0x00000094"),"{log}");
    }
}

#[cfg(test)] mod v57_boot_branch_trace {
    use super::*;
    #[test] fn opcode_419a0088_is_decoded_and_condition_evaluated(){
        let base=0x8214_AB10u64;
        let mut session=Session::create(vec![0x41,0x9A,0x00,0x88,0x60,0,0,0],base,base,
            &[base as i64,base as i64+8]).unwrap();
        let report=session.step_chunk(1);
        assert!(report.contains("DISASM=bc BO=12 BI=26"),"{report}");
        assert!(report.contains("taken=false"),"{report}");
        assert!(report.contains("INSTRUCAO #0: PC=0x8214AB10"),"{report}");
    }
}

#[cfg(test)] mod v58_boot_compatibility {
    use super::*;
    fn minecraft()->Session{
        Session::create(vec![0x41,0x9A,0x00,0x88,0x60,0,0,0],0x8214_AB10,0x8214_AB10,
            &[0x8214_AB10,0x8214_AB18]).unwrap()
    }
    #[test] fn exact_title_and_entry_enable_cr6_eq_and_take_first_branch(){
        let mut session=minecraft();
        assert!(session.configure_boot_cr(0x4D53_0A81).unwrap().contains("CR6.EQ=1"));
        assert_eq!(session.cpu.cr,0x20);
        // Branch target leaves this tiny synthetic exec region; the CPU still
        // executes the first bc correctly, then validation catches next fetch.
        session.step_chunk(1);
        assert_eq!(session.cpu.pc,0x8214_AB98);
    }
    #[test] fn no_global_cr_minecraft_hack(){
        let mut session=minecraft();
        assert!(session.configure_boot_cr(0x4D53_0A82).unwrap().starts_with("OK|"));
        assert_eq!(session.cpu.cr,0);
        let mut changed=minecraft();
        changed.cpu.pc=0x8214_AB14;
        changed.configure_boot_cr(0x4D53_0A81).unwrap();
        assert_eq!(changed.cpu.cr,0);
        let mut bad=minecraft();
        bad.mem.load_image_bytes(0x8214_AB10,&0x60000000u32.to_be_bytes()).unwrap();
        bad.configure_boot_cr(0x4D53_0A81).unwrap();
        assert_eq!(bad.cpu.cr,0);
    }
}
