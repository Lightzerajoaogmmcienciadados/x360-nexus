//! X360 Nexus experimental PowerPC interpreter; not a game emulator.
#[derive(Debug)]
pub struct XenonCpu {
    pub gpr: [u64; 32],
    pub pc: u64,
    pub memory: Vec<u8>,
}
impl XenonCpu {
    pub fn new(size: usize) -> Self {
        Self { gpr: [0; 32], pc: 0, memory: vec![0; size] }
    }
    pub fn load(&mut self, address: usize, data: &[u8]) -> Result<(), String> {
        let end = address.checked_add(data.len()).ok_or("address overflow")?;
        let dest = self.memory.get_mut(address..end).ok_or("out of bounds")?;
        dest.copy_from_slice(data);
        self.pc = address as u64;
        Ok(())
    }
    pub fn step(&mut self) -> Result<(), String> {
        let address = usize::try_from(self.pc).map_err(|_| "invalid pc")?;
        if address % 4 != 0 { return Err("unaligned pc".into()); }
        let end = address.checked_add(4).ok_or("pc overflow")?;
        let bytes: [u8; 4] = self.memory.get(address..end)
            .ok_or("instruction out of bounds")?.try_into().unwrap();
        let insn = u32::from_be_bytes(bytes);
        let opcode = insn >> 26;
        let rs_or_rt = ((insn >> 21) & 31) as usize;
        let ra = ((insn >> 16) & 31) as usize;
        let imm = (insn as u16 as i16) as i64;
        match opcode {
            14 => { // addi rD,rA,SIMM (rA=0 means zero)
                let base = if ra == 0 { 0 } else { self.gpr[ra] };
                self.gpr[rs_or_rt] = base.wrapping_add(imm as u64);
            }
            24 => { // ori rA,rS,UIMM
                self.gpr[ra] = self.gpr[rs_or_rt] | (insn as u16 as u64);
            }
            _ => return Err(format!("unsupported opcode {opcode}")),
        }
        self.pc = self.pc.checked_add(4).ok_or("pc overflow")?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn addi_and_ori() {
        let mut cpu = XenonCpu::new(64);
        cpu.load(0, &[0x38, 0x60, 0x00, 0x2a, 0x60, 0x64, 0x00, 0x01]).unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.gpr[3], 42);
        cpu.step().unwrap();
        assert_eq!(cpu.gpr[4], 43);
        assert_eq!(cpu.pc, 8);
    }
    #[test]
    fn rejects_invalid_access() {
        let mut cpu = XenonCpu::new(4);
        assert!(cpu.load(3, &[1, 2]).is_err());
    }
}
