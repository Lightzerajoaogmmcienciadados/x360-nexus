//! Cooperative round-robin scheduler for original test programs. This does not
//! reproduce Xenon's three cores, six hardware threads or Xbox kernel ABI.
use crate::cpu::Cpu;
use std::collections::VecDeque;

#[derive(Debug,Clone)]
pub struct ScheduledThread {pub id:u32,pub cpu:Cpu}
#[derive(Debug)]
pub struct Scheduler {
    queue:VecDeque<ScheduledThread>,
    current_id:u32,
    next_id:u32,
    quantum:u32,
    ticks_in_quantum:u32,
}
impl Scheduler {
    pub fn new()->Self{Self{queue:VecDeque::new(),current_id:1,next_id:2,quantum:64,ticks_in_quantum:0}}
    pub fn threads(&self)->usize{1+self.queue.len()}
    pub fn current_id(&self)->u32{self.current_id}
    pub fn next_guest_id(&self)->u32{self.next_id}
    pub fn spawn(&mut self,pc:usize)->Result<u32,String>{
        let mut cpu=Cpu::new();cpu.pc=pc;
        self.spawn_cpu(cpu)
    }
    pub fn spawn_cpu(&mut self,cpu:Cpu)->Result<u32,String>{
        if self.queue.len()>=7{return Err("maximum 8 diagnostic guest threads".into());}
        let id=self.next_id;self.next_id=self.next_id.saturating_add(1);
        self.queue.push_back(ScheduledThread{id,cpu});Ok(id)
    }
    pub fn tick(&mut self,current:&mut Cpu){
        self.ticks_in_quantum+=1;
        if self.ticks_in_quantum<self.quantum || self.queue.is_empty(){return;}
        self.ticks_in_quantum=0;
        if let Some(next)=self.queue.pop_front(){
            let old=std::mem::replace(current,next.cpu);
            self.queue.push_back(ScheduledThread{id:self.current_id,cpu:old});
            self.current_id=next.id;
        }
    }
}
impl Default for Scheduler{fn default()->Self{Self::new()}}
#[cfg(test)] mod tests {
    use super::*;
    #[test]fn round_robin(){
        let mut s=Scheduler::new();let mut cpu=Cpu::new();cpu.pc=0x100;
        let id=s.spawn(0x200).unwrap();
        for _ in 0..64{s.tick(&mut cpu);}
        assert_eq!(cpu.pc,0x200);assert_eq!(s.current_id(),id);
        for _ in 0..64{s.tick(&mut cpu);}
        assert_eq!(cpu.pc,0x100);assert_eq!(s.current_id(),1);
    }
}