//! Núcleo experimental PowerPC, independente do APK.
#[derive(Debug)]
pub struct Cpu { pub gpr: [u64; 32], pub pc: usize, pub ram: Vec<u8> }
impl Cpu {
    pub fn new(size: usize) -> Self { Self { gpr: [0; 32], pc: 0, ram: vec![0; size] } }
    pub fn load(&mut self, code: &[u8]) -> Result<(), &'static str> {
        if code.len() > self.ram.len() { return Err("memory overflow"); }
        self.ram[..code.len()].copy_from_slice(code);
        self.pc = 0;
        Ok(())
    }
    pub fn step(&mut self) -> Result<(), &'static str> {
        let b = self.ram.get(self.pc..self.pc.checked_add(4).ok_or("invalid pc")?).ok_or("out of bounds")?;
        let ins = u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
        let op = ins >> 26;
        let rt = ((ins >> 21) & 31) as usize;
        let ra = ((ins >> 16) & 31) as usize;
        match op {
            14 => { // addi
                let base = if ra == 0 { 0 } else { self.gpr[ra] };
                let imm = (ins as u16 as i16 as i64) as u64;
                self.gpr[rt] = base.wrapping_add(imm);
            }
            24 => { // ori: bits 21..25 = source, 16..20 = destination
                self.gpr[ra] = self.gpr[rt] | (ins as u16 as u64);
            }
            _ => return Err("unsupported instruction"),
        }
        self.pc += 4;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn addi_42() { let mut cpu = Cpu::new(128); cpu.load(&[0x38,0x60,0,42]).unwrap(); cpu.step().unwrap(); assert_eq!(cpu.gpr[3],42); }
    #[test] fn ori_mask() { let mut cpu = Cpu::new(128); cpu.gpr[3]=0x100; cpu.load(&[0x60,0x64,0,0x0f]).unwrap(); cpu.step().unwrap(); assert_eq!(cpu.gpr[4],0x10f); }
}
