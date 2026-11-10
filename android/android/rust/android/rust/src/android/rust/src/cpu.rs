
pub struct XenonCpu {
    pub gpr: [u64; 32],
    pub pc: u64,
    pub memory: Vec<u8>,
    pub running: bool,
}

impl XenonCpu {
    pub fn new(memory_size: usize) -> Self {
        Self {
            gpr: [0; 32],
            pc: 0,
            memory: vec![0; memory_size],
            running: false,
        }
    }

    pub fn load_program(
        &mut self,
        address: usize,
        program: &[u8],
    ) -> Result<(), String> {
        let end = address
            .checked_add(program.len())
            .ok_or("Endereco invalido")?;

        if end > self.memory.len() {
            return Err("Memoria insuficiente".into());
        }

        self.memory[address..end].copy_from_slice(program);
        self.pc = address as u64;
        Ok(())
    }

    pub fn step(&mut self) -> Result<(), String> {
        let pc = usize::try_from(self.pc)
            .map_err(|_| "PC invalido")?;

        let end = pc.checked_add(4)
            .ok_or("PC invalido")?;

        let bytes = self.memory.get(pc..end)
            .ok_or("Instrucao fora da memoria")?;

        let instruction = u32::from_be_bytes(
            bytes.try_into().unwrap()
        );

        let opcode = instruction >> 26;
        let rd = ((instruction >> 21) & 31) as usize;
        let ra = ((instruction >> 16) & 31) as usize;
        let imm = (instruction as u16 as i16) as i64;

        match opcode {
            // addi - PowerPC
            14 => {
                let base = if ra == 0 {
                    0
                } else {
                    self.gpr[ra]
                };

                self.gpr[rd] =
                    base.wrapping_add(imm as u64);
            }

            // ori - PowerPC
            24 => {
                let rs = rd;
                let dest = ra;
                self.gpr[dest] =
                    self.gpr[rs] | (instruction as u16 as u64);
            }

            _ => {
                return Err(format!(
                    "Opcode nao implementado: {}",
                    opcode
                ));
            }
        }

        self.pc += 4;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executes_addi() {
        let mut cpu = XenonCpu::new(1024);

        // addi r3, r0, 42
        cpu.load_program(
            0,
            &[0x38, 0x60, 0x00, 0x2A]
        ).unwrap();

        cpu.step().unwrap();

        assert_eq!(cpu.gpr[3], 42);
        assert_eq!(cpu.pc, 4);
    }
}
