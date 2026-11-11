use crate::memory::Memory;

#[derive(Debug)]
pub struct Cpu {
    pub gpr: [u64; 32],
    pub pc: usize,
    pub lr: u64,
    pub ctr: u64,
    pub halted: bool,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            gpr: [0; 32],
            pc: 0,
            lr: 0,
            ctr: 0,
            halted: false,
        }
    }

    fn address(
        &self,
        ra: usize,
        offset: i64,
    ) -> Result<usize, String> {
        let base = if ra == 0 {
            0
        } else {
            self.gpr[ra]
        };

        let addr = base.wrapping_add(offset as u64);

        usize::try_from(addr)
            .map_err(|_| "address overflow".to_string())
    }

    pub fn step(
        &mut self,
        mem: &mut Memory,
    ) -> Result<(), String> {
        if self.halted {
            return Err("CPU halted".into());
        }

        let ins = mem.read_u32_be(self.pc)
            .map_err(str::to_string)?;

        let op = ins >> 26;
        let rt = ((ins >> 21) & 31) as usize;
        let ra = ((ins >> 16) & 31) as usize;

        let imm = (ins as u16 as i16) as i64;

        let mut next = self.pc.checked_add(4)
            .ok_or("PC overflow")?;

        match op {
            // mulli
            7 => {
                self.gpr[rt] =
                    (self.gpr[ra] as i64)
                    .wrapping_mul(imm) as u64;
            }

            // subfic (carry ainda não modelado)
            8 => {
                self.gpr[rt] =
                    (imm as u64)
                    .wrapping_sub(self.gpr[ra]);
            }

            // addic (carry ainda não modelado)
            12 => {
                self.gpr[rt] =
                    self.gpr[ra]
                    .wrapping_add(imm as u64);
            }

            // addi
            14 => {
                let base = if ra == 0 {
                    0
                } else {
                    self.gpr[ra]
                };

                self.gpr[rt] =
                    base.wrapping_add(imm as u64);
            }

            // addis
            15 => {
                let base = if ra == 0 {
                    0
                } else {
                    self.gpr[ra]
                };

                self.gpr[rt] = base.wrapping_add(
                    (imm << 16) as u64
                );
            }

            // Branch
            18 => {
                let disp =
                    ((ins & 0x03ff_fffc) as i32)
                    << 6 >> 6;

                let target = if ins & 2 != 0 {
                    disp as i64
                } else {
                    (self.pc as i64)
                        .wrapping_add(disp as i64)
                };

                if target < 0 {
                    return Err(
                        "negative branch target".into()
                    );
                }

                if ins & 1 != 0 {
                    self.lr = next as u64;
                }

                next = target as usize;
            }

            // ori
            24 => {
                self.gpr[ra] =
                    self.gpr[rt] | (ins as u16 as u64);
            }

            // oris
            25 => {
                self.gpr[ra] =
                    self.gpr[rt] |
                    ((ins as u16 as u64) << 16);
            }

            // xori
            26 => {
                self.gpr[ra] =
                    self.gpr[rt] ^ (ins as u16 as u64);
            }

            // andi. (CR0 ainda não modelado)
            28 => {
                self.gpr[ra] =
                    self.gpr[rt] & (ins as u16 as u64);
            }

            // lwz
            32 => {
                let addr = self.address(ra, imm)?;

                self.gpr[rt] =
                    mem.read_u32_be(addr)
                    .map_err(str::to_string)? as u64;
            }

            // lbz
            34 => {
                let addr = self.address(ra, imm)?;

                self.gpr[rt] =
                    mem.read_u8(addr)
                    .map_err(str::to_string)? as u64;
            }

            // stw
            36 => {
                let addr = self.address(ra, imm)?;

                mem.write_u32_be(
                    addr,
                    self.gpr[rt] as u32
                ).map_err(str::to_string)?;
            }

            // stb
            38 => {
                let addr = self.address(ra, imm)?;

                mem.write_u8(
                    addr,
                    self.gpr[rt] as u8
                ).map_err(str::to_string)?;
            }

            _ => {
                return Err(format!(
                    "unsupported PowerPC opcode {} at 0x{:X}",
                    op,
                    self.pc
                ));
            }
        }

        self.pc = next;
        Ok(())
    }

    pub fn run_steps(
        &mut self,
        mem: &mut Memory,
        limit: usize,
    ) -> Result<usize, String> {
        for i in 0..limit {
            self.step(mem)?;

            if self.halted {
                return Ok(i + 1);
            }
        }

        Ok(limit)
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_addi() {
        let mut mem = Memory::new(32);

        mem.write(
            0,
            &[0x38, 0x60, 0x00, 0x2A]
        ).unwrap();

        let mut cpu = Cpu::new();
        cpu.step(&mut mem).unwrap();

        assert_eq!(cpu.gpr[3], 42);
    }

    #[test]
    fn test_ori() {
        let mut mem = Memory::new(32);

        mem.write(
            0,
            &[0x60, 0x64, 0x00, 0x0F]
        ).unwrap();

        let mut cpu = Cpu::new();
        cpu.gpr[3] = 256;

        cpu.step(&mut mem).unwrap();

        assert_eq!(cpu.gpr[4], 271);
    }

    #[test]
    fn test_load_store_word() {
        let mut mem = Memory::new(128);

        // stw r3, 64(r0)
        mem.write_u32_be(
            0, 0x90600040
        ).unwrap();

        // lwz r4, 64(r0)
        mem.write_u32_be(
            4, 0x80800040
        ).unwrap();

        let mut cpu = Cpu::new();
        cpu.gpr[3] = 0x12345678;

        cpu.step(&mut mem).unwrap();
        cpu.step(&mut mem).unwrap();

        assert_eq!(
            cpu.gpr[4],
            0x12345678
        );

        assert_eq!(
            mem.read_u32_be(64).unwrap(),
            0x12345678
        );
    }

    #[test]
    fn test_load_store_byte() {
        let mut mem = Memory::new(128);

        // stb r3, 64(r0)
        mem.write_u32_be(
            0, 0x98600040
        ).unwrap();

        // lbz r4, 64(r0)
        mem.write_u32_be(
            4, 0x88800040
        ).unwrap();

        let mut cpu = Cpu::new();
        cpu.gpr[3] = 0xABCD;

        cpu.step(&mut mem).unwrap();
        cpu.step(&mut mem).unwrap();

        assert_eq!(cpu.gpr[4], 0xCD);
    }

    #[test]
    fn test_memory_bounds() {
        let mut mem = Memory::new(16);

        mem.write_u32_be(
            0, 0x80600040
        ).unwrap();

        let mut cpu = Cpu::new();

        assert!(cpu.step(&mut mem).is_err());
    }

    #[test]
    fn test_unsupported() {
        let mut mem = Memory::new(16);
        let mut cpu = Cpu::new();

        assert!(cpu.step(&mut mem).is_err());
    }
}
