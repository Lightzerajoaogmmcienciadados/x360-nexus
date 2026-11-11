use std::collections::BTreeMap;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]pub enum ThreadState{Ready,Waiting,Stopped}
#[derive(Debug)]pub struct Kernel{next_id:u32,threads:BTreeMap<u32,ThreadState>,ticks:u64}
impl Kernel{pub fn new()->Self{Self{next_id:1,threads:BTreeMap::new(),ticks:0}}pub fn create_thread(&mut self)->u32{let id=self.next_id;self.next_id+=1;self.threads.insert(id,ThreadState::Ready);id}pub fn set_thread_state(&mut self,id:u32,state:ThreadState)->Result<(),&'static str>{*self.threads.get_mut(&id).ok_or("unknown thread")?=state;Ok(())}pub fn state(&self,id:u32)->Option<ThreadState>{self.threads.get(&id).copied()}pub fn tick(&mut self){self.ticks=self.ticks.wrapping_add(1)}pub fn ticks(&self)->u64{self.ticks}}
impl Default for Kernel{fn default()->Self{Self::new()}}
