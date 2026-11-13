//! Sandbox HLE helpers for self-authored PowerPC programs.
//! These service IDs are NOT the ordinal/ABI of a real xboxkrnl.xex or xam.xex.
use crate::memory::Memory;
use std::collections::{BTreeMap,BTreeSet};

#[derive(Debug,Clone,Copy,PartialEq,Eq)]pub enum ThreadState{Ready,Waiting,Stopped}
#[derive(Debug)]
pub struct Kernel{
    next_id:u32,threads:BTreeMap<u32,ThreadState>,ticks:u64,
    next_tls:u32,tls:BTreeMap<u32,u64>,next_heap:u64,
    allocations:BTreeMap<u64,usize>,critical:BTreeMap<u64,u32>,
    safe_stubs:BTreeSet<(String,u32)>,events:BTreeMap<u64,bool>, video_ready:bool,
    swaps:u64,display_mode:(u32,u32),
    gpu_ring_addr:u64, gpu_ring_dwords:u32,
    gpu_pending_addr:u64, gpu_pending_dwords:u32,
    last_pad:crate::input::Controller,input_packet:u32,
}
impl Kernel {
    pub fn new()->Self{Self{next_id:1,threads:BTreeMap::new(),ticks:0,next_tls:0,tls:BTreeMap::new(),next_heap:0,allocations:BTreeMap::new(),critical:BTreeMap::new(),safe_stubs:BTreeSet::new(),events:BTreeMap::new(),video_ready:false,swaps:0,display_mode:(1280,720),gpu_ring_addr:0,gpu_ring_dwords:0,gpu_pending_addr:0,gpu_pending_dwords:0,last_pad:crate::input::Controller::default(),input_packet:0}}
    pub fn create_thread(&mut self)->u32{let id=self.next_id;self.next_id=self.next_id.saturating_add(1);self.threads.insert(id,ThreadState::Ready);id}
    pub fn set_thread_state(&mut self,id:u32,state:ThreadState)->Result<(),&'static str>{*self.threads.get_mut(&id).ok_or("unknown thread")?=state;Ok(())}
    pub fn state(&self,id:u32)->Option<ThreadState>{self.threads.get(&id).copied()}
    pub fn tick(&mut self){self.ticks=self.ticks.wrapping_add(1)}
    pub fn ticks(&self)->u64{self.ticks}
    /// Reserve a low guest heap region for boot diagnostics. Only actual
    /// allocations map pages; all other low addresses remain unmapped.
    pub fn set_guest_heap_base(&mut self,base:u64)->Result<(),String>{
        if base<0x0100_0000||base>=0x7000_0000||base&4095!=0 {
            return Err("invalid low guest heap base".into());
        }
        self.next_heap=base;
        Ok(())
    }
    pub fn gpu_ring_snapshot(&self)->(u64,u32,u64){
        (if self.gpu_pending_dwords!=0 {self.gpu_pending_addr}else{self.gpu_ring_addr},
         if self.gpu_pending_dwords!=0{self.gpu_pending_dwords}else{self.gpu_ring_dwords},
         self.swaps)
    }
    pub fn capture_gpu_ring_write(&mut self,write:Option<(u32,u32)>){
        self.gpu_pending_addr=write.map(|v|v.0 as u64).unwrap_or(0);
        self.gpu_pending_dwords=write.map(|v|v.1).unwrap_or(0);
    }
    pub fn register_secondary_stub(&mut self,library:&str,ordinal:u32)->Result<(),String>{
        if !matches!(library,"xboxkrnl.exe"|"xam.xex") || ordinal==0 {return Err("invalid HLE stub registration".into())}
        self.safe_stubs.insert((library.to_owned(),ordinal));Ok(())
    }
    pub fn dispatch_secondary_stub(&self,library:&str,ordinal:u32,gpr:&mut [u64;32])->Result<String,String>{
        if !self.safe_stubs.contains(&(library.to_owned(),ordinal)) {return Err(format!("UNRESOLVED critical/import {library} ordinal {ordinal}"));}
        gpr[3]=0;
        Ok(format!("WARNING: synthetic stub {library} ordinal={ordinal} returned r3=0 (unimplemented)"))
    }
    /// Calls are addressed by a private test ABI, not Xbox ordinals.
    /// Arguments use r3..r10 and guest pointers in the bounded Memory snapshot.
    pub fn dispatch_private(&mut self,id:u32,gpr:&mut [u64;32],mem:&mut Memory)->Result<String,String>{
        let a=gpr[3] as usize;
        let status=match id {
            1 => { // NtAllocateVirtualMemory(base_ptr=r3, size_ptr=r4)
                let size=mem.read_u32_be(gpr[4] as usize).map_err(str::to_string)? as usize;
                if size==0 || size>16*1024*1024{return Err("allocation size invalid".into());}
                if self.next_heap==0 { self.next_heap=(mem.base()+if mem.is_sparse(){128*1024*1024}else{0x10000}+4095)&!4095; }
                let start=(self.next_heap+4095)&!4095;
                let rounded=(size as u64+4095)&!4095;
                if start.checked_add(rounded).is_none_or(|end|end>mem.base()+mem.len() as u64){return Err("out of guest memory".into());}
                self.next_heap=start+rounded;
                mem.set_page_permissions(start,rounded as usize,Memory::READ|Memory::WRITE).map_err(str::to_string)?;
                mem.write_u32_be(a,start as u32).map_err(str::to_string)?;
                self.allocations.insert(start,size);
                "NtAllocateVirtualMemory"
            },
            2 => { // NtFreeVirtualMemory: removes allocation tracking only
                let addr=mem.read_u32_be(a).map_err(str::to_string)? as u64;
                let bytes=self.allocations.remove(&addr).ok_or("unknown virtual allocation")?;
                mem.unmap_region(addr,(bytes+4095)&!4095).map_err(str::to_string)?;
                "NtFreeVirtualMemory"
            },
            3 => {let slot=self.next_tls;self.next_tls=self.next_tls.saturating_add(1);self.tls.insert(slot,0);gpr[3]=slot as u64;return Ok(format!("KeTlsAlloc -> {slot}"));},
            4 => {let val=*self.tls.get(&(gpr[3] as u32)).ok_or("KeTlsGetValue invalid slot")?;gpr[3]=val;return Ok(format!("KeTlsGetValue -> {val}"));},
            5 => {let slot=gpr[3] as u32;*self.tls.get_mut(&slot).ok_or("KeTlsSetValue invalid slot")?=gpr[4];"KeTlsSetValue"},
            6 => {let id=self.create_thread();if a!=0{mem.write_u32_be(a,id).map_err(str::to_string)?;}"ExCreateThread (record only; not scheduled)"},
            7 => {self.critical.insert(gpr[3],0);"KeInitializeCriticalSection"},
            8 => {let depth=self.critical.get_mut(&gpr[3]).ok_or("uninitialized critical section")?;*depth=depth.saturating_add(1);"RtlEnterCriticalSection"},
            9 => {let depth=self.critical.get_mut(&gpr[3]).ok_or("uninitialized critical section")?;if *depth==0{return Err("critical section not held".into())};*depth-=1;"RtlLeaveCriticalSection"},
            10=>{mem.write_u32_be(a,1280).map_err(str::to_string)?;mem.write_u32_be(a+4,720).map_err(str::to_string)?;mem.write_u32_be(a+8,60).map_err(str::to_string)?;mem.write_u32_be(a+12,32).map_err(str::to_string)?;"VdQueryVideoMode (synthetic 720p descriptor)"},
            11=>{self.events.insert(gpr[3],gpr[4]!=0);"KeInitializeEvent (test ABI)"},
            12=>{*self.events.get_mut(&gpr[3]).ok_or("event not initialized")?=true;"KeSetEvent (test ABI)"},
            13=>{
                let event=self.events.get_mut(&gpr[3]).ok_or("event not initialized")?;
                if !*event{gpr[3]=0x102;return Ok("KeWaitForSingleObject: STATUS_TIMEOUT (nonblocking test)".into());}
                *event=false;"KeWaitForSingleObject"
            },
            14=>{ // MmAllocatePhysicalMemory (synthetic): returns offset inside snapshot.
                let size=(gpr[3] as usize).max(1);
                if size>1024*1024{return Err("physical allocation exceeds diagnostic limit".into());}
                if self.next_heap==0{self.next_heap=(mem.base()+if mem.is_sparse(){128*1024*1024}else{0x10000}+4095)&!4095;}
                let start=(self.next_heap+4095)&!4095;
                let end=start.checked_add(((size as u64)+4095)&!4095).ok_or("out of memory")?;
                if end>mem.base()+mem.len() as u64{return Err("out of guest memory".into());}
                self.next_heap=end;self.allocations.insert(start,size);
                mem.set_page_permissions(start,((size+4095)&!4095),Memory::READ|Memory::WRITE).map_err(str::to_string)?;
                gpr[3]=start;return Ok(format!("MmAllocatePhysicalMemory test -> 0x{start:X}"));
            },
            15=>{ // VdSwap: only counts submitted frames, no real Xenos content.
                if !self.video_ready{return Err("VdSwap: engine not initialized".into());}
                self.swaps=self.swaps.saturating_add(1);
                "VdSwap (test frame counter only)"
            },
            16=>return Err("NtOpenFile requires guest filesystem handles; not implemented".into()),
            17=>return Err("NtReadFile requires guest filesystem handles; not implemented".into()),
            18=>{self.video_ready=true;"VdInitializeEngines (diagnostic only)"},
            19=>{ // VdSetDisplayMode: a mode request, not an Xenos output mode.
                let (w,h)=(gpr[3] as u32,gpr[4] as u32);
                if !(320..=3840).contains(&w)||!(240..=2160).contains(&h){return Err("unsupported synthetic display mode".into());}
                self.display_mode=(w,h);"VdSetDisplayMode (request logged only)"
            },
            20=>{ // Private demo ABI: return a guest-mapped PM4 command ring.
                // Reserved 64KiB in the 512MiB guest window; real Xenos uses
                // graphics MMIO and write pointers which are NOT emulated.
                if !mem.is_sparse(){return Err("VdGetSystemCommandBuffer needs sparse guest RAM".into());}
                if self.gpu_ring_addr==0 {
                    let at=0x9F00_0000u64;
                    mem.set_page_permissions(at,65536,Memory::READ|Memory::WRITE)
                        .map_err(str::to_string)?;
                    self.gpu_ring_addr=at;
                }
                mem.track_gpu_ring(self.gpu_ring_addr as usize,65536)
                    .map_err(str::to_string)?;
                gpr[3]=self.gpu_ring_addr;
                gpr[4]=65536;
                return Ok(format!("VdGetSystemCommandBuffer test ring -> 0x{:X} (64KiB)",self.gpu_ring_addr));
            },
            21=>{ // Private test ABI: submit DWORD count for the mapped ring.
                let count=gpr[3] as u32;
                if self.gpu_ring_addr==0||count==0||count>4096{return Err("PM4 submit: no ring or invalid word count".into());}
                self.gpu_ring_dwords=count;
                "SubmitGpuRing (test ABI; read at VdSwap only)"
            },
            22=>{ // Local offline profile. Never fake a signed-in Xbox Live user.
                gpr[3]=0;
                return Ok("XamUserGetSigninState: offline local test user".into());
            },
            23=>{std::thread::yield_now();"NtYieldExecution (host cooperative test)"},
            _=>return Err(format!("unknown private HLE service {id}")),
        };
        gpr[3]=0;
        Ok(format!("{status}: synthetic HLE status=0"))
    }
}
// Real XEX2 import-thunk ABI: only these verified ordinals have a
// narrowly scoped implementation. Other ordinals must stop with diagnostics.
impl Kernel {
    fn xex_virtual_alloc(&mut self,gpr:&mut [u64;32],mem:&mut Memory)->Result<String,String>{
        // Xbox ABI: base-address pointer, region-size pointer,
        // allocation type, protection flags, debug memory flag.
        let base_ptr=gpr[3] as usize;
        let size_ptr=gpr[4] as usize;
        let allocation_type=gpr[5] as u32;
        let _protect=gpr[6] as u32;
        let debug=gpr[7];
        let desired=mem.read_u32_be(base_ptr).map_err(str::to_string)? as u64;
        let size=mem.read_u32_be(size_ptr).map_err(str::to_string)? as u64;
        if desired!=0 || debug!=0 || size==0 || size>16*1024*1024 || allocation_type&0x3000==0 {
            gpr[3]=0xC000_000D; // STATUS_INVALID_PARAMETER
            return Ok("NtAllocateVirtualMemory unsupported fixed address/flags: STATUS_INVALID_PARAMETER".into());
        }
        let rounded=(size+4095)&!4095;
        let base=(self.next_heap+4095)&!4095;
        let end=base.checked_add(rounded).ok_or("virtual heap overflow")?;
        if base<0x2000_0000 || end>0x4000_0000 {
            gpr[3]=0xC000_0017; // STATUS_NO_MEMORY
            return Ok("NtAllocateVirtualMemory: STATUS_NO_MEMORY".into());
        }
        mem.set_page_permissions(base,rounded as usize,Memory::READ|Memory::WRITE)
            .map_err(str::to_string)?;
        self.next_heap=end;
        mem.write_u32_be(base_ptr,base as u32).map_err(str::to_string)?;
        mem.write_u32_be(size_ptr,rounded as u32).map_err(str::to_string)?;
        self.allocations.insert(base,rounded as usize);
        gpr[3]=0;
        Ok(format!("NtAllocateVirtualMemory mapped 0x{base:08X}+0x{rounded:X} in low guest VA"))
    }
    fn xex_virtual_free(&mut self,gpr:&mut [u64;32],mem:&mut Memory)->Result<String,String>{
        let base_ptr=gpr[3] as usize;
        let size_ptr=gpr[4] as usize;
        let addr=mem.read_u32_be(base_ptr).map_err(str::to_string)? as u64;
        let alloc=self.allocations.get(&addr).copied();
        if gpr[6]!=0 || alloc.is_none(){
            gpr[3]=0xC000_000D;
            return Ok(format!("NtFreeVirtualMemory invalid guest allocation 0x{addr:X}"));
        }
        let size=alloc.unwrap();
        mem.unmap_region(addr,size).map_err(str::to_string)?;
        self.allocations.remove(&addr);
        mem.write_u32_be(size_ptr,size as u32).map_err(str::to_string)?;
        gpr[3]=0;
        Ok(format!("NtFreeVirtualMemory unmapped 0x{addr:08X}+0x{size:X}"))
    }
    /// Verified xboxkrnl ordinals; only a narrow subset of ABI behaviors is
    /// modeled. Unsupported xboxkrnl/xam exports fail explicitly.
    pub fn dispatch_xex_thunk(&mut self,module:u32,ordinal:u32,
        gpr:&mut [u64;32],mem:&mut Memory)->Result<String,String>{
        if module==2 {
            return match ordinal {
                0x210=>{ // XamUserGetSigninState: no fabricated Xbox Live account.
                    gpr[3]=0;
                    Ok("XamUserGetSigninState: offline (not signed in)".into())
                },
                0x191=>{ // XamInputGetState(user, flags, X_INPUT_STATE*).
                    // X_INPUT_STATE is a 4-byte packet count + 12-byte X_GAMEPAD,
                    // with big-endian multi-byte members in the PowerPC guest.
                    let user=gpr[3];let dest=gpr[5] as usize;
                    if user!=0 {gpr[3]=1167;return Ok("XamInputGetState: user not connected".into());}
                    if dest==0 {gpr[3]=87;return Ok("XamInputGetState: invalid pointer".into());}
                    let pad=crate::galaxy_jni::controller_snapshot();
                    if self.last_pad!=pad {
                        self.input_packet=self.input_packet.wrapping_add(1);
                        self.last_pad=pad;
                    }
                    mem.write_u32_be(dest,self.input_packet).map_err(str::to_string)?;
                    mem.write_u16_be(dest+4,pad.buttons as u16).map_err(str::to_string)?;
                    mem.write_u8(dest+6,pad.left_trigger).map_err(str::to_string)?;
                    mem.write_u8(dest+7,pad.right_trigger).map_err(str::to_string)?;
                    for (i,axis) in [pad.left_x,pad.left_y,pad.right_x,pad.right_y].iter().enumerate(){
                        mem.write_u16_be(dest+8+i*2,*axis as u16).map_err(str::to_string)?;
                    }
                    gpr[3]=0;
                    Ok(format!("XamInputGetState: packet {}",self.input_packet))
                },
                _=>Err(format!("xam.xex ordinal {ordinal}: HLE nao implementado")),
            };
        }
        if module!=1{return Err(format!("unknown import module {module}"));}
        match ordinal {
            131=>{ // KeQueryPerformanceFrequency: Xenon 50 MHz guest timebase.
                gpr[3]=50_000_000;
                Ok("KeQueryPerformanceFrequency: 50000000 Hz (guest)".into())
            },
            132=>{ // KeQuerySystemTime: writes Windows FILETIME to guest pointer.
                // This is a bounded host-time approximation, not kernel timers.
                let dest=gpr[3] as usize;
                let elapsed=std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|e|format!("host clock before Unix epoch: {e}"))?;
                let ticks=(elapsed.as_secs()+11_644_473_600u64).saturating_mul(10_000_000)
                    .saturating_add(u64::from(elapsed.subsec_nanos()/100));
                mem.write_u64_be(dest,ticks).map_err(str::to_string)?;
                gpr[3]=0;
                Ok(format!("KeQuerySystemTime: wrote FILETIME to 0x{dest:08X}"))
            },
            13=>self.dispatch_private(6,gpr,mem), // ExCreateThread: runtime schedules entry at r7
            204=>self.xex_virtual_alloc(gpr,mem),
            220=>self.xex_virtual_free(gpr,mem),
            293=>self.dispatch_private(8,gpr,mem), // RtlEnterCriticalSection
            302=>self.dispatch_private(7,gpr,mem), // RtlInitializeCriticalSection
            304=>self.dispatch_private(9,gpr,mem), // RtlLeaveCriticalSection
            338=>self.dispatch_private(3,gpr,mem), // KeTlsAlloc
            340=>self.dispatch_private(4,gpr,mem), // KeTlsGetValue
            341=>self.dispatch_private(5,gpr,mem), // KeTlsSetValue
            112=>self.dispatch_private(11,gpr,mem), // KeInitializeEvent, simplified
            157=>self.dispatch_private(12,gpr,mem), // KeSetEvent, simplified
            176=>self.dispatch_private(13,gpr,mem), // KeWaitForSingleObject, nonblocking
            467=>self.dispatch_private(19,gpr,mem), // VdSetDisplayMode (synthetic)
            450=>self.dispatch_private(18,gpr,mem), // VdInitializeEngines (diagnostic)
            458=>self.dispatch_private(10,gpr,mem), // VdQueryVideoMode (diagnostic)
            445=>self.dispatch_private(20,gpr,mem), // VdGetSystemCommandBuffer (test ring)
            603=>self.dispatch_private(15,gpr,mem), // VdSwap (test frame counter)
            257=>self.dispatch_private(23,gpr,mem), // NtYieldExecution
            _=>{
                let name=match ordinal {
                    13=>"ExCreateThread",112=>"KeInitializeEvent",
                    185=>"MmAllocatePhysicalMemory",186=>"MmAllocatePhysicalMemoryEx",
                    210=>"NtCreateFile",223=>"NtOpenFile",224=>"NtOpenSymbolicLinkObject",
                    293=>"RtlEnterCriticalSection",302=>"RtlInitializeCriticalSection",
                    304=>"RtlLeaveCriticalSection",407=>"XexGetProcedureAddress",
                    445=>"VdGetSystemCommandBuffer",450=>"VdInitializeEngines",
                    467=>"VdSetDisplayMode",603=>"VdSwap",_=>"(unknown export)"
                };
                Err(format!("xboxkrnl.exe ordinal {ordinal} {name}: HLE nao implementado"))
            },
        }
    }
}

impl Default for Kernel{fn default()->Self{Self::new()}}
#[cfg(test)]mod tests {
 use super::*;
 #[test]fn tls_and_safe_imports(){let mut k=Kernel::new();let mut m=Memory::new(1024*1024);let mut g=[0u64;32];k.dispatch_private(3,&mut g,&mut m).unwrap();let slot=g[3];g[3]=slot;g[4]=55;k.dispatch_private(5,&mut g,&mut m).unwrap();g[3]=slot;k.dispatch_private(4,&mut g,&mut m).unwrap();assert_eq!(g[3],55);assert!(k.dispatch_secondary_stub("xam.xex",99,&mut g).is_err());k.register_secondary_stub("xam.xex",99).unwrap();assert!(k.dispatch_secondary_stub("xam.xex",99,&mut g).unwrap().contains("WARNING"));}
 #[test]fn events_and_video(){
    let mut k=Kernel::new();let mut m=Memory::new(4096);let mut regs=[0u64;32];
    regs[3]=0x100;k.dispatch_private(11,&mut regs,&mut m).unwrap();
    regs[3]=0x100;let result=k.dispatch_private(13,&mut regs,&mut m).unwrap();
    assert!(result.contains("TIMEOUT"));assert_eq!(regs[3],0x102);
    regs[3]=0x100;k.dispatch_private(12,&mut regs,&mut m).unwrap();
    regs[3]=0x100;k.dispatch_private(13,&mut regs,&mut m).unwrap();assert_eq!(regs[3],0);
    k.dispatch_private(18,&mut regs,&mut m).unwrap();k.dispatch_private(15,&mut regs,&mut m).unwrap();
 }
 #[test]fn gpu_ring_private_abi(){
   let mut k=Kernel::new();
   let mut m=Memory::sparse(0x8200_0000,512*1024*1024).unwrap();
   let mut g=[0u64;32];
   assert!(k.dispatch_private(20,&mut g,&mut m).unwrap().contains("CommandBuffer"));
   assert_eq!(g[3],0x9F00_0000);
   m.write(g[3] as usize,&[0xC0,0,0,0]).unwrap();
   g[3]=1; k.dispatch_private(21,&mut g,&mut m).unwrap();
   k.dispatch_private(18,&mut g,&mut m).unwrap();
   k.dispatch_private(15,&mut g,&mut m).unwrap();
   assert_eq!(k.gpu_ring_snapshot(),(0x9F00_0000,1,1));
 }
 #[test]fn virtual_memory(){let mut k=Kernel::new();let mut m=Memory::with_base(vec![0;1024*1024],0x82000000);let mut g=[0u64;32];g[3]=0x82000100;g[4]=0x82000104;m.write_u32_be(g[4] as usize,4096).unwrap();k.dispatch_private(1,&mut g,&mut m).unwrap();assert_eq!(g[3],0);assert!(m.read_u32_be(0x82000100).unwrap()>=0x82010000);}
}

#[cfg(test)] mod real_abi_tests {
    use super::*;
    #[test] fn allocate_free_uses_guest_be_pointers(){
        let mut mem=Memory::sparse(0x8200_0000,512*1024*1024).unwrap();
        mem.set_page_permissions(0x70010000,4096,Memory::READ|Memory::WRITE).unwrap();
        let mut kernel=Kernel::new();kernel.set_guest_heap_base(0x20000000).unwrap();
        let mut regs=[0u64;32];
        regs[3]=0x70010000;regs[4]=0x70010004;regs[5]=0x3000;regs[6]=4;
        mem.write_u32_be(0x70010000,0).unwrap();
        mem.write_u32_be(0x70010004,8192).unwrap();
        let msg=kernel.dispatch_xex_thunk(1,204,&mut regs,&mut mem).unwrap();
        assert!(msg.contains("mapped"),"{msg}");assert_eq!(regs[3],0);
        let addr=mem.read_u32_be(0x70010000).unwrap() as usize;
        assert_eq!(addr,0x20000000);
        mem.write_u32_be(addr,0xDEADBEEF).unwrap();
        assert_eq!(mem.read_u32_be(addr).unwrap(),0xDEADBEEF);
        regs[3]=0x70010000;regs[4]=0x70010004;regs[5]=0x8000;regs[6]=0;
        kernel.dispatch_xex_thunk(1,220,&mut regs,&mut mem).unwrap();
        assert_eq!(regs[3],0);assert!(mem.read_u32_be(addr).is_err());
    }
}

#[cfg(test)] mod v56_time_tests {
  use super::*;
  #[test] fn frequency_and_system_time_write_to_guest_big_endian() {
    let mut mem=Memory::new(4096);
    let mut k=Kernel::new();let mut g=[0u64;32];
    k.dispatch_xex_thunk(1,131,&mut g,&mut mem).unwrap();
    assert_eq!(g[3],50_000_000);
    g[3]=128;
    k.dispatch_xex_thunk(1,132,&mut g,&mut mem).unwrap();
    assert!(mem.read_u64_be(128).unwrap()>133_000_000_000_000_000);
  }
}