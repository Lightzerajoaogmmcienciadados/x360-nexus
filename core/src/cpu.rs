use crate::memory::Memory;
#[derive(Debug)]pub struct Cpu {pub gpr:[u64;32],pub pc:usize,pub lr:u64,pub ctr:u64,pub halted:bool}
impl Cpu {
 pub fn new()->Self{Self{gpr:[0;32],pc:0,lr:0,ctr:0,halted:false}}
 pub fn step(&mut self,mem:&Memory)->Result<(),String>{
  if self.halted{return Err("CPU halted".into())}
  let ins=mem.read_u32_be(self.pc).map_err(str::to_string)?;
  let op=ins>>26;let rt=((ins>>21)&31) as usize;let ra=((ins>>16)&31) as usize;
  let mut next=self.pc.checked_add(4).ok_or("PC overflow")?;
  match op {
   14=>{let base=if ra==0{0}else{self.gpr[ra]};self.gpr[rt]=base.wrapping_add((ins as u16 as i16 as i64) as u64);},
   15=>{let base=if ra==0{0}else{self.gpr[ra]};let val=((ins as u16 as i16 as i64)<<16) as u64;self.gpr[rt]=base.wrapping_add(val);},
   24=>{self.gpr[ra]=self.gpr[rt]|(ins as u16 as u64);},
   25=>{self.gpr[ra]=self.gpr[rt]|((ins as u16 as u64)<<16);},
   18=>{ // unconditional relative/absolute branch (b)
    let disp=((ins & 0x03ff_fffc) as i32)<<6>>6;
    let target=if (ins&2)!=0 {disp as i64} else {(self.pc as i64).wrapping_add(disp as i64)};
    if target<0{return Err("negative branch target".into())}
    if (ins&1)!=0{self.lr=next as u64;}
    next=target as usize;
   },
   
   7 => {
       // mulli: multiplicação por imediato
       let imm = (ins as u16 as i16) as i64;
       let value = self.gpr[ra] as i64;
       self.gpr[rt] = value.wrapping_mul(imm) as u64;
   },

   8 => {
       // subfic: imediato menos registrador
       let imm = (ins as u16 as i16) as i64;
       self.gpr[rt] =
           (imm as u64).wrapping_sub(self.gpr[ra]);
   },

   12 => {
       // addic: adição com imediato
       let imm = (ins as u16 as i16) as i64;
       self.gpr[rt] =
           self.gpr[ra].wrapping_add(imm as u64);
   },

   26 => {
       // xori: XOR imediato
       self.gpr[ra] =
           self.gpr[rt] ^ (ins as u16 as u64);
   },

   28 => {
       // andi.: AND imediato
       self.gpr[ra] =
           self.gpr[rt] & (ins as u16 as u64);
   },
   _=>return Err(format!("unsupported PowerPC opcode {op} at 0x{:X}",self.pc)),
  }
  self.pc=next;Ok(())
 }
 pub fn run_steps(&mut self,mem:&Memory,limit:usize)->Result<usize,String>{for i in 0..limit{self.step(mem)?;if self.halted{return Ok(i+1)}}Ok(limit)}
}
impl Default for Cpu {fn default()->Self{Self::new()}}
#[cfg(test)]mod tests {use super::*;
 #[test]fn addi(){let mut m=Memory::new(32);m.write(0,&[0x38,0x60,0,42]).unwrap();let mut c=Cpu::new();c.step(&m).unwrap();assert_eq!(c.gpr[3],42);}
 #[test]fn ori(){let mut m=Memory::new(32);m.write(0,&[0x60,0x64,0,15]).unwrap();let mut c=Cpu::new();c.gpr[3]=256;c.step(&m).unwrap();assert_eq!(c.gpr[4],271);}
 #[test]fn unsupported(){let m=Memory::new(16);assert!(Cpu::new().step(&m).is_err());}
}
