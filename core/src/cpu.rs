//! Experimental big-endian PowerPC instruction interpreter (tiny, bounded subset).
//! Not a Xenon 360 three-core simulator, JIT, MMU or complete PowerPC ISA.
use crate::memory::Memory;

#[derive(Debug)]
pub struct Cpu {
    pub gpr: [u64; 32],
    pub pc: usize,
    pub lr: u64,
    pub ctr: u64,
    pub cr: u32,
    pub halted: bool,
    pub steps: u64,
}
impl Cpu {
    pub fn new() -> Self {
        Self { gpr: [0; 32], pc: 0, lr: 0, ctr: 0, cr: 0, halted: false, steps: 0 }
    }
    fn dword(&self, ra: usize, imm: i16) -> Result<usize, String> {
        let base = if ra == 0 { 0 } else { self.gpr[ra] };
        usize::try_from(base.wrapping_add(imm as i64 as u64))
            .map_err(|_| "effective address outside host".to_owned())
    }
    fn set_cr0_unsigned(&mut self, lhs: u64, rhs: u64) {
        let field = if lhs < rhs {8} else if lhs > rhs {4} else {2};
        self.cr = (self.cr & 0x0fff_ffff) | (field << 28);
    }
    fn set_cr0(&mut self, lhs: i64, rhs: i64) {
        let field = if lhs < rhs { 8 } else if lhs > rhs { 4 } else { 2 };
        self.cr = (self.cr & 0x0fff_ffff) | (field << 28);
    }
    pub fn step(&mut self, memory: &mut Memory) -> Result<(), String> {
        if self.halted { return Err("CPU is halted".into()); }
        let ins = memory.read_exec_u32_be(self.pc).map_err(str::to_owned)?;
        let op = ins >> 26;
        let rt = ((ins >> 21) & 31) as usize;
        let ra = ((ins >> 16) & 31) as usize;
        let rb = ((ins >> 11) & 31) as usize;
        let immediate = ins as u16 as i16;
        let mut next = self.pc.checked_add(4).ok_or("PC overflow")?;
        match op {
            7 => { // mulli
                self.gpr[rt] = self.gpr[ra].wrapping_mul(immediate as i64 as u64);
            }
            8 => { // subfic
                self.gpr[rt] = (immediate as i64 as u64).wrapping_sub(self.gpr[ra]);
            }
            14 => {
                let base = if ra == 0 { 0 } else { self.gpr[ra] };
                self.gpr[rt] = base.wrapping_add(immediate as i64 as u64);
            }
            15 => {
                let base = if ra == 0 { 0 } else { self.gpr[ra] };
                self.gpr[rt] = base.wrapping_add(((immediate as i64) << 16) as u64);
            }
            24 => self.gpr[ra] = self.gpr[rt] | (ins as u16 as u64),
            25 => self.gpr[ra] = self.gpr[rt] | ((ins as u16 as u64) << 16),
            26 => self.gpr[ra] = self.gpr[rt] ^ (ins as u16 as u64),
            27 => self.gpr[ra] = self.gpr[rt] ^ ((ins as u16 as u64) << 16),
            28 => {
                self.gpr[ra] = self.gpr[rt] & (ins as u16 as u64);
                self.set_cr0(self.gpr[ra] as i64, 0);
            }
            10 => self.set_cr0_unsigned(self.gpr[ra] as u32 as u64, ins as u16 as u64),
            11 => self.set_cr0(self.gpr[ra] as i32 as i64, immediate as i64),
            29 => {
                self.gpr[ra] = self.gpr[rt] & ((ins as u16 as u64) << 16);
                self.set_cr0(self.gpr[ra] as i64, 0);
            },
            32 => {
                let addr = self.dword(ra, immediate)?;
                self.gpr[rt] = memory.read_u32_be(addr).map_err(str::to_owned)? as u64;
            }
            34 => {
                let addr = self.dword(ra, immediate)?;
                self.gpr[rt] = memory.read(addr, 1).map_err(str::to_owned)?[0] as u64;
            }
            36 => {
                let addr = self.dword(ra, immediate)?;
                memory.write(addr, &(self.gpr[rt] as u32).to_be_bytes()).map_err(str::to_owned)?;
            }
            38 => {
                let addr = self.dword(ra, immediate)?;
                memory.write(addr, &[self.gpr[rt] as u8]).map_err(str::to_owned)?;
            }
            40 => {
                let addr = self.dword(ra, immediate)?;
                let bytes = memory.read(addr, 2).map_err(str::to_owned)?;
                self.gpr[rt] = u16::from_be_bytes([bytes[0], bytes[1]]) as u64;
            }
            44 => {
                let addr = self.dword(ra, immediate)?;
                memory.write(addr, &(self.gpr[rt] as u16).to_be_bytes()).map_err(str::to_owned)?;
            }
            18 => {
                let disp = ((ins & 0x03ff_fffc) as i32) << 6 >> 6;
                let target = if (ins & 2) != 0 {
                    disp as i64
                } else {
                    (self.pc as i64).wrapping_add(disp as i64)
                };
                if target < 0 { return Err("negative branch target".into()); }
                if (ins & 1) != 0 { self.lr = next as u64; }
                next = target as usize;
            }
            16 => {
                let bo = (ins >> 21) & 31;
                let bi = (ins >> 16) & 31;
                // Supports common CR-only beq/bne-style conditions; CTR modes unsupported.
                let cr_bit = self.cr & (1u32 << (31 - bi)) != 0;
                let taken = match bo { 20 => true, 12 => cr_bit, 4 => !cr_bit,
                    _ => return Err(format!("unsupported branch option BO={bo}")) };
                if taken {
                    let disp = ((ins & 0xfffc) as i16) as i64;
                    let target = if (ins & 2) != 0 { disp } else { (self.pc as i64).wrapping_add(disp) };
                    if target < 0 { return Err("negative conditional branch".into()); }
                    next = target as usize;
                }
                if (ins & 1) != 0 { self.lr = self.pc as u64 + 4; }
            }
            19 => {
                let xo = (ins >> 1) & 1023;
                if xo != 16 || (ins >> 21) & 31 != 20 {
                    return Err(format!("unsupported branch control XO={xo}"));
                }
                next = (self.lr & !3) as usize;
            }
            31 => {
                let xo = (ins >> 1) & 1023;
                match xo {
                    266 => self.gpr[rt] = self.gpr[ra].wrapping_add(self.gpr[rb]),
                    235 => self.gpr[rt] = (self.gpr[ra] as i32)
                        .wrapping_mul(self.gpr[rb] as i32) as i64 as u64,
                    104 => self.gpr[rt] = (self.gpr[ra] as i64).wrapping_neg() as u64,
                    954 => self.gpr[ra] = (self.gpr[rt] as i8 as i64) as u64,
                    922 => self.gpr[ra] = (self.gpr[rt] as i16 as i64) as u64,
                    339 => {
                        let spr = ((ins >> 16) & 31) | (((ins >> 11) & 31) << 5);
                        self.gpr[rt] = match spr {
                            8 => self.lr, 9 => self.ctr,
                            _ => return Err(format!("unsupported mfspr SPR={spr}")),
                        };
                    }
                    467 => {
                        let spr = ((ins >> 16) & 31) | (((ins >> 11) & 31) << 5);
                        match spr {
                            8 => self.lr=self.gpr[rt],
                            9 => self.ctr=self.gpr[rt],
                            _ => return Err(format!("unsupported mtspr SPR={spr}")),
                        }
                    },
                    40 => self.gpr[rt] = self.gpr[rb].wrapping_sub(self.gpr[ra]),
                    444 => self.gpr[ra] = self.gpr[rt] | self.gpr[rb],
                    28 => self.gpr[ra] = self.gpr[rt] & self.gpr[rb],
                    316 => self.gpr[ra] = self.gpr[rt] ^ self.gpr[rb],
                    0 => self.set_cr0(self.gpr[ra] as i32 as i64, self.gpr[rb] as i32 as i64),
                    _ => return Err(format!("unsupported PowerPC extended opcode {xo} at 0x{:x}", self.pc)),
                }
            }
            _ => return Err(format!("unsupported PowerPC opcode {op} at 0x{:x}", self.pc)),
        }
        if next % 4 != 0 { return Err("unaligned instruction address".into()); }
        self.pc = next;
        self.steps += 1;
        Ok(())
    }
    pub fn run_steps(&mut self, memory: &mut Memory, limit: usize) -> Result<usize, String> {
        if limit > 1_000_000 { return Err("step budget exceeded".into()); }
        for i in 0..limit {
            self.step(memory)?;
            if self.halted { return Ok(i + 1); }
        }
        Ok(limit)
    }
}
impl Default for Cpu { fn default() -> Self { Self::new() } }
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn addi_ori() {
        let mut mem = Memory::new(64);
        mem.write(0, &[0x38,0x60,0,42,0x60,0x64,0,15]).unwrap();
        let mut cpu=Cpu::new();cpu.step(&mut mem).unwrap();cpu.step(&mut mem).unwrap();
        assert_eq!(cpu.gpr[3],42);assert_eq!(cpu.gpr[4],47);
    }
    #[test] fn load_store() {
        let mut mem=Memory::new(128);
        mem.write(0,&[0x38,0x60,0,42,0x90,0x60,0,64,0x80,0x80,0,64]).unwrap();
        let mut cpu=Cpu::new();cpu.run_steps(&mut mem,3).unwrap();
        assert_eq!(cpu.gpr[4],42);
        assert_eq!(mem.read_u32_be(64).unwrap(),42);
    }
    #[test] fn multiply_immediate_and_subtract() {
        let mut mem=Memory::new(32);
        // li r3,7; mulli r4,r3,6; subfic r5,r4,50
        mem.write(0,&[0x38,0x60,0,7, 0x1c,0x83,0,6, 0x20,0xa4,0,50]).unwrap();
        let mut cpu=Cpu::new();cpu.run_steps(&mut mem,3).unwrap();
        assert_eq!(cpu.gpr[4],42);assert_eq!(cpu.gpr[5],8);
    }
    #[test] fn link_register_move() {
        // mflr r3, mtlr r3
        let mut mem=Memory::new(32);
        mem.write(0,&[0x7c,0x68,0x02,0xa6,0x7c,0x68,0x03,0xa6]).unwrap();
        let mut c=Cpu::new();c.lr=0x1234;
        c.step(&mut mem).unwrap();assert_eq!(c.gpr[3],0x1234);
        c.lr=0;c.step(&mut mem).unwrap();assert_eq!(c.lr,0x1234);
    }
    #[test] fn rejects_unknown_opcode() {
        let mut mem=Memory::new(8);mem.write(0,&[0xff,0xff,0xff,0xff]).unwrap();
        assert!(Cpu::new().step(&mut mem).is_err());
    }
}
