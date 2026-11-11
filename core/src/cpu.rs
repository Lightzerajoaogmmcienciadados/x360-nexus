
use crate::memory::Memory;

#[derive(Debug)]
pub struct Cpu {
    pub gpr: [u64; 32],
    pub pc: usize,
    pub lr: u64,
    pub ctr: u64,
    pub cr: u32,
    pub halted: bool,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            gpr: [0; 32],
            pc: 0,
            lr: 0,
            ctr: 0,
            cr: 0,
            halted: false,
        }
    }

    fn set_cr0(&mut self, value: i64) {
        let field = if value < 0 {
            0b1000
        } else if value > 0 {
            0b0100
        } else {
            0b0010
        };

        self.cr = (self.cr & 0x0FFF_FFFF)
            | (field << 28);
    }

    pub fn step(
        &mut self,
        mem: &Memory,
    ) -> Result<(), String> {
        if self.halted {
            return Err("CPU parada".into());
        }

        let ins = mem
            .read_u32_be(self.pc)
            .map_err(str::to_string)?;

        let op = ins >> 26;
        let rt = ((ins >> 21) & 31) as usize;
        let ra = ((ins >> 16) & 31) as usize;
        let rb = ((ins >> 11) & 31) as usize;

        let mut next = self.pc
            .checked_add(4)
            .ok_or("PC overflow")?;

        match op {
            // addi
            14 => {
                let base = if ra == 0 {
                    0
                } else {
                    self.gpr[ra]
                };

                let imm = ins as u16 as i16 as i64;
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

                let imm =
                    (ins as u16 as i16 as i64) << 16;

                self.gpr[rt] =
                    base.wrapping_add(imm as u64);
            }

            // ori
            24 => {
                self.gpr[ra] =
                    self.gpr[rt] | (ins as u16 as u64);
            }

            // oris
            25 => {
                self.gpr[ra] =
                    self.gpr[rt]
                    | ((ins as u16 as u64) << 16);
            }

            // xori
            26 => {
                self.gpr[ra] =
                    self.gpr[rt] ^ (ins as u16 as u64);
            }

            // andi.
            28 => {
                self.gpr[ra] =
                    self.gpr[rt] & (ins as u16 as u64);

                self.set_cr0(self.gpr[ra] as i64);
            }

            // b
            18 => {
                let disp =
                    ((ins & 0x03FF_FFFC) as i32)
                    << 6 >> 6;

                let target = if ins & 2 != 0 {
                    disp as i64
                } else {
                    (self.pc as i64)
                        .wrapping_add(disp as i64)
                };

                if target < 0 {
                    return Err(
                        "Destino negativo".into()
                    );
                }

                if ins & 1 != 0 {
                    self.lr = next as u64;
                }

                next = target as usize;
            }

            // cmpi (signed)
            11 => {
                let l = (ins >> 21) & 1;
                let bf = ((ins >> 23) & 7) as u32;

                let a = if l == 0 {
                    self.gpr[ra] as u32 as i32 as i64
                } else {
                    self.gpr[ra] as i64
                };

                let b = ins as u16 as i16 as i64;

                let field = if a < b {
                    0b1000u32
                } else if a > b {
                    0b0100
                } else {
                    0b0010
                };

                let shift = 28 - bf * 4;
                let mask = 0xFu32 << shift;

                self.cr =
                    (self.cr & !mask)
                    | (field << shift);
            }

            // bc (conditional branch)
            16 => {
                let bo = (ins >> 21) & 31;
                let bi = (ins >> 16) & 31;

                let disp =
                    ((ins & 0xFFFC) as i16) as i64;

                let mut ctr_ok = true;
                if bo & 4 == 0 {
                    self.ctr = self.ctr.wrapping_sub(1);
                    ctr_ok = (self.ctr != 0)
                        ^ (bo & 2 != 0);
                }

                let cr_bit =
                    (self.cr >> (31 - bi)) & 1;

                let cond_ok = bo & 16 != 0
                    || cr_bit == ((bo >> 3) & 1);

                if ctr_ok && cond_ok {
                    let target = if ins & 2 != 0 {
                        disp
                    } else {
                        (self.pc as i64)
                            .wrapping_add(disp)
                    };

                    if target < 0 {
                        return Err(
                            "Destino negativo".into()
                        );
                    }

                    next = target as usize;
                }

                if ins & 1 != 0 {
                    self.lr = (self.pc + 4) as u64;
                }
            }

            // Grupo de instrucoes opcode 31
            31 => {
                let xo = (ins >> 1) & 1023;

                match xo {
                    // add
                    266 => {
                        self.gpr[rt] =
                            self.gpr[ra]
                            .wrapping_add(self.gpr[rb]);

                        if ins & 1 != 0 {
                            self.set_cr0(
                                self.gpr[rt] as i64
                            );
                        }
                    }

                    // subf
                    40 => {
                        self.gpr[rt] =
                            self.gpr[rb]
                            .wrapping_sub(self.gpr[ra]);

                        if ins & 1 != 0 {
                            self.set_cr0(
                                self.gpr[rt] as i64
                            );
                        }
                    }

                    // or
                    444 => {
                        self.gpr[ra] =
                            self.gpr[rt] | self.gpr[rb];

                        if ins & 1 != 0 {
                            self.set_cr0(
                                self.gpr[ra] as i64
                            );
                        }
                    }

                    _ => {
                        return Err(format!(
                            "Instrucao XO {} nao suportada",
                            xo
                        ));
                    }
                }
            }

            // Marcador privado para testes.
            // Nao representa uma instrucao
            // PowerPC de jogos comerciais.
            0 => {
                if ins == 0 {
                    self.halted = true;
                } else {
                    return Err(
                        "Instrucao desconhecida".into()
                    );
                }
            }

            _ => {
                return Err(format!(
                    "Opcode PowerPC {} nao suportado \
                     em 0x{:X}",
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
        mem: &Memory,
        limit: usize,
    ) -> Result<usize, String> {
        let mut executed = 0;

        while executed < limit && !self.halted {
            self.step(mem)?;
            executed += 1;
        }

        Ok(executed)
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
    fn execute_program() {
        let mut mem = Memory::new(64);

        mem.write(0, &[
            0x38, 0x60, 0x00, 0x2A,
            0x38, 0x83, 0x00, 0x08,
            0x00, 0x00, 0x00, 0x00,
        ]).unwrap();

        let mut cpu = Cpu::new();

        let count = cpu.run_steps(&mem, 100).unwrap();

        assert_eq!(cpu.gpr[3], 42);
        assert_eq!(cpu.gpr[4], 50);
        assert_eq!(count, 3);
        assert!(cpu.halted);
    }

    #[test]
    fn unsupported_opcode() {
        let mut mem = Memory::new(16);

        mem.write(0, &[0xFF, 0xFF, 0xFF, 0xFF])
            .unwrap();

        assert!(Cpu::new().step(&mem).is_err());
    }
}
