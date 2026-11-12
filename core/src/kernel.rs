use std::collections::BTreeMap;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]pub enum ThreadState{Ready,Waiting,Stopped}
#[derive(Debug)]pub struct Kernel{next_id:u32,threads:BTreeMap<u32,ThreadState>,ticks:u64}
impl Kernel{pub fn new()->Self{Self{next_id:1,threads:BTreeMap::new(),ticks:0}}pub fn create_thread(&mut self)->u32{let id=self.next_id;self.next_id+=1;self.threads.insert(id,ThreadState::Ready);id}pub fn set_thread_state(&mut self,id:u32,state:ThreadState)->Result<(),&'static str>{*self.threads.get_mut(&id).ok_or("unknown thread")?=state;Ok(())}pub fn state(&self,id:u32)->Option<ThreadState>{self.threads.get(&id).copied()}pub fn tick(&mut self){self.ticks=self.ticks.wrapping_add(1)}pub fn ticks(&self)->u64{self.ticks}}

impl Kernel {
    /// Cooperative diagnostic scheduler; not the Xbox kernel scheduler.
    pub fn ready_count(&self)->usize{
        self.threads.values().filter(|&&s|s==ThreadState::Ready).count()
    }
    pub fn next_ready(&self,after:Option<u32>)->Option<u32>{
        if self.threads.is_empty(){return None;}
        let candidates:Vec<u32>=self.threads.iter().filter_map(
            |(&id,&state)|if state==ThreadState::Ready{Some(id)}else{None}).collect();
        if candidates.is_empty(){return None;}
        let index=after.and_then(|last|candidates.iter().position(|&id|id>last))
            .unwrap_or(0);
        Some(candidates[index])
    }
}
impl Default for Kernel{fn default()->Self{Self::new()}}
#[cfg(test)] mod scheduler_tests{
    use super::*;
    #[test]fn rotates_ready_threads_and_skips_waiting(){
        let mut k=Kernel::new();
        let a=k.create_thread();let b=k.create_thread();let c=k.create_thread();
        k.set_thread_state(b,ThreadState::Waiting).unwrap();
        assert_eq!(k.ready_count(),2);
        assert_eq!(k.next_ready(None),Some(a));
        assert_eq!(k.next_ready(Some(a)),Some(c));
        assert_eq!(k.next_ready(Some(c)),Some(a));
        k.set_thread_state(c,ThreadState::Stopped).unwrap();
        assert_eq!(k.next_ready(Some(a)),Some(a));
    }
}

