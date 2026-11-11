//! Bounded PowerPC interpreter connected to the sparse virtual memory.
//! Only a subset of instructions for original homebrew; no XEX execution.
use crate::virtual_memory::{VirtualMemory, READ, WRITE, EXEC, PAGE_SIZE};

pub struct ExecutionEngine {
    pub memory: VirtualMemory,
    pub registers: [u64; 32],
    pub pc: u32,
    pub ticks: u64,
    pub halted: bool,
}
impl ExecutionEngine {
    pub fn new() -> Self { Self {memory:VirtualMemory::new(),registers:[0;32],pc:0,ticks:0,halted:false} }
    pub fn load_homebrew(&mut self,base:u32,code:&[u8])->Result<(),String>{
        if code.is_empty() || code.len()>1024*1024 || code.len()%4!=0 || base as usize%PAGE_SIZE!=0 {return Err("invalid homebrew size/alignment".into())}
        let size=code.len().checked_add(PAGE_SIZE-1).ok_or("size overflow")? / PAGE_SIZE * PAGE_SIZE;
        self.memory.map(base,size,READ|WRITE|EXEC)?;
        self.memory.write(base,code)?;
        self.pc=base;
        Ok(())
    }
    pub fn step(&mut self)->Result<(),String>{
        if self.halted{return Err("CPU halted".into())}
        if self.pc%4!=0{return Err("unaligned instruction".into())}
        let instruction=self.memory.read_u32_be(self.pc,EXEC)?;
        let op=instruction>>26;
        let rt=((instruction>>21)&31) as usize;
        let ra=((instruction>>16)&31) as usize;
        let rb=((instruction>>11)&31) as usize;
        let mut next=self.pc.checked_add(4).ok_or("PC overflow")?;
        match op {
            14|15=>{
                let base=if ra==0{0}else{self.registers[ra]};
                let imm=(instruction as u16 as i16 as i64) as u64;
                let add=if op==15{imm.wrapping_shl(16)}else{imm};
                self.registers[rt]=base.wrapping_add(add);
            }
            24=>{self.registers[ra]=self.registers[rt] | (instruction as u16 as u64);}
            26=>{self.registers[ra]=self.registers[rt] ^ (instruction as u16 as u64);}
            18=>{
                let displacement=((instruction & 0x03ff_fffc) as i32)<<6>>6;
                let target=if instruction&2!=0{displacement as i64}else{(self.pc as i64)+displacement as i64};
                next=u32::try_from(target).map_err(|_|"branch out of range")?;
            }
            31 if ((instruction>>1)&1023)==266=>{
                self.registers[rt]=self.registers[ra].wrapping_add(self.registers[rb]);
            }
            0 if instruction==0=>{self.halted=true;}
            _=>return Err(format!("unsupported PowerPC opcode 0x{instruction:08X} at 0x{:08X}",self.pc)),
        }
        self.pc=next;
        self.ticks+=1;
        Ok(())
    }
    pub fn run(&mut self,max_steps:usize)->Result<usize,String>{
        if max_steps==0 || max_steps>100_000{return Err("invalid execution budget".into())}
        let mut count=0;
        while !self.halted && count<max_steps {self.step()?;count+=1;}
        if !self.halted {return Err(format!("execution budget exceeded after {count} steps"))}
        Ok(count)
    }
}
impl Default for ExecutionEngine {fn default()->Self{Self::new()}}
pub fn demo()->Result<String,String>{
    let mut vm=ExecutionEngine::new();
    // addi r3,r0,42; addi r4,r3,8; add r5,r3,r4; halt (private test sentinel).
    let code=[0x38,0x60,0,42, 0x38,0x83,0,8, 0x7c,0xa3,0x22,0x14, 0,0,0,0];
    vm.load_homebrew(0x1000,&code)?;
    let steps=vm.run(64)?;
    if vm.registers[3]!=42 || vm.registers[4]!=50 || vm.registers[5]!=92{return Err("register self-test failed".into())}
    Ok(format!("Execution Engine OK: {steps} steps, r3={}, r4={}, r5={}, PC=0x{:X}, pages={}",vm.registers[3],vm.registers[4],vm.registers[5],vm.pc,vm.memory.mapped_pages()))
}
#[cfg(test)] mod tests {use super::*;
    #[test]fn execution(){assert!(demo().unwrap().contains("r5=92"));}
    #[test]fn budget(){let mut vm=ExecutionEngine::new();vm.load_homebrew(0x1000,&[0x48,0,0,0]).unwrap();assert!(vm.run(5).unwrap_err().contains("budget"));}
    #[test]fn unsupported(){let mut vm=ExecutionEngine::new();vm.load_homebrew(0x1000,&[0xff,0xff,0xff,0xff]).unwrap();assert!(vm.step().is_err());}
    #[test]fn alignment(){let mut vm=ExecutionEngine::new();assert!(vm.load_homebrew(1,&[0,0,0,0]).is_err());}
}
