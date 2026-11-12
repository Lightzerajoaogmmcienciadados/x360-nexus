use crate::memory::Memory;

/// Interpreter skeleton for a small subset of 64-bit PowerPC Xenon instructions.
#[derive(Debug)]
pub struct Cpu {
    pub gpr: [u64;32],
    pub pc: usize,
    pub lr: u64,
    pub ctr: u64,
    pub halted: bool,
}
impl Cpu {
    pub fn new() -> Self { Self {gpr:[0;32],pc:0,lr:0,ctr:0,halted:false} }
    pub fn step(&mut self, memory:&Memory) -> Result<(),String> {
        if self.halted{return Err("CPU halted".into());}
        let instr=memory.read_u32_be(self.pc).map_err(str::to_string)?;
        let op=instr>>26;
        let rt=((instr>>21)&31) as usize;
        let ra=((instr>>16)&31) as usize;
        let rb=((instr>>11)&31) as usize;
        let mut next=self.pc.checked_add(4).ok_or("PC overflow")?;
        match op {
            14 => { // addi RT, RA, SIMM
                let base=if ra==0 {0} else {self.gpr[ra]};
                self.gpr[rt]=base.wrapping_add((instr as u16 as i16 as i64) as u64);
            }
            15 => { // addis
                let base=if ra==0 {0} else {self.gpr[ra]};
                self.gpr[rt]=base.wrapping_add(((instr as u16 as i16 as i64)<<16) as u64);
            }
            24 => self.gpr[ra]=self.gpr[rt] | (instr as u16 as u64), // ori
            25 => self.gpr[ra]=self.gpr[rt] | ((instr as u16 as u64)<<16), // oris
            31 => { // selected extended integer operations
                let xo=(instr>>1)&1023;
                if xo==266 { // add RT,RA,RB (OE bit clear)
                    self.gpr[rt]=self.gpr[ra].wrapping_add(self.gpr[rb]);
                } else {
                    return Err(format!("PowerPC opcode 31 XO={xo} ainda sem suporte em 0x{:X}",self.pc));
                }
            }
            18 => { // b/bl with 26-bit signed displacement
                let disp=((instr&0x03FF_FFFC) as i32)<<6>>6;
                let target=if instr&2!=0 {disp as i64} else {(self.pc as i64).wrapping_add(disp as i64)};
                if target<0 {return Err("negative branch target".into());}
                if instr&1!=0 {self.lr=next as u64;}
                next=target as usize;
            }
            17 => return Err(format!("sc syscall nao implementada; r0={}",self.gpr[0])),
            _ => return Err(format!("unsupported PowerPC opcode {op} at 0x{:X}",self.pc)),
        }
        self.pc=next;
        Ok(())
    }
    pub fn run_steps(&mut self, mem:&Memory, limit:usize)->Result<usize,String>{
        for i in 0..limit {self.step(mem)?; if self.halted{return Ok(i+1);}}
        Ok(limit)
    }
}
impl Default for Cpu{fn default()->Self{Self::new()}}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn addi(){let mut m=Memory::new(32);m.write(0,&[0x38,0x60,0,42]).unwrap();let mut c=Cpu::new();c.step(&m).unwrap();assert_eq!(c.gpr[3],42);}
    #[test]fn ori(){let mut m=Memory::new(32);m.write(0,&[0x60,0x64,0,15]).unwrap();let mut c=Cpu::new();c.gpr[3]=256;c.step(&m).unwrap();assert_eq!(c.gpr[4],271);}
    #[test]fn add(){let mut m=Memory::new(32);m.write(0,&[0x7c,0xa3,0x22,0x14]).unwrap();let mut c=Cpu::new();c.gpr[3]=5;c.gpr[4]=7;c.step(&m).unwrap();assert_eq!(c.gpr[5],12);}
}