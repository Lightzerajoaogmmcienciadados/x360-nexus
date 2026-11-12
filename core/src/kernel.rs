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
    safe_stubs:BTreeSet<(String,u32)>,
}
impl Kernel {
    pub fn new()->Self{Self{next_id:1,threads:BTreeMap::new(),ticks:0,next_tls:0,tls:BTreeMap::new(),next_heap:0,allocations:BTreeMap::new(),critical:BTreeMap::new(),safe_stubs:BTreeSet::new()}}
    pub fn create_thread(&mut self)->u32{let id=self.next_id;self.next_id=self.next_id.saturating_add(1);self.threads.insert(id,ThreadState::Ready);id}
    pub fn set_thread_state(&mut self,id:u32,state:ThreadState)->Result<(),&'static str>{*self.threads.get_mut(&id).ok_or("unknown thread")?=state;Ok(())}
    pub fn state(&self,id:u32)->Option<ThreadState>{self.threads.get(&id).copied()}
    pub fn tick(&mut self){self.ticks=self.ticks.wrapping_add(1)}
    pub fn ticks(&self)->u64{self.ticks}
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
                if self.next_heap==0 { self.next_heap=(mem.base()+0x10000+4095)&!4095; }
                let start=(self.next_heap+4095)&!4095;
                let rounded=(size as u64+4095)&!4095;
                if start.checked_add(rounded).is_none_or(|end|end>mem.base()+mem.len() as u64){return Err("out of guest memory".into());}
                self.next_heap=start+rounded;
                mem.write_u32_be(a,start as u32).map_err(str::to_string)?;
                self.allocations.insert(start,size);
                "NtAllocateVirtualMemory"
            },
            2 => { // NtFreeVirtualMemory: removes allocation tracking only
                let addr=mem.read_u32_be(a).map_err(str::to_string)? as u64;
                if self.allocations.remove(&addr).is_none(){return Err("unknown virtual allocation".into());}
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
            _=>return Err(format!("unknown private HLE service {id}")),
        };
        gpr[3]=0;
        Ok(format!("{status}: synthetic HLE status=0"))
    }
}
impl Default for Kernel{fn default()->Self{Self::new()}}
#[cfg(test)]mod tests {
 use super::*;
 #[test]fn tls_and_safe_imports(){let mut k=Kernel::new();let mut m=Memory::new(1024*1024);let mut g=[0u64;32];k.dispatch_private(3,&mut g,&mut m).unwrap();let slot=g[3];g[3]=slot;g[4]=55;k.dispatch_private(5,&mut g,&mut m).unwrap();g[3]=slot;k.dispatch_private(4,&mut g,&mut m).unwrap();assert_eq!(g[3],55);assert!(k.dispatch_secondary_stub("xam.xex",99,&mut g).is_err());k.register_secondary_stub("xam.xex",99).unwrap();assert!(k.dispatch_secondary_stub("xam.xex",99,&mut g).unwrap().contains("WARNING"));}
 #[test]fn virtual_memory(){let mut k=Kernel::new();let mut m=Memory::with_base(vec![0;1024*1024],0x82000000);let mut g=[0u64;32];g[3]=0x82000100;g[4]=0x82000104;m.write_u32_be(g[4] as usize,4096).unwrap();k.dispatch_private(1,&mut g,&mut m).unwrap();assert_eq!(g[3],0);assert!(m.read_u32_be(0x82000100).unwrap()>=0x82010000);}
}