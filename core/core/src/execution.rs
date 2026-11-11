
use crate::cpu::Cpu;
use crate::memory::Memory;

pub struct ExecutionEngine {
    pub cpu: Cpu,
    pub memory: Memory,
    pub instructions_executed: u64,
}

impl ExecutionEngine {
    pub fn new(memory_size: usize) -> Self {
        Self {
            cpu: Cpu::new(),
            memory: Memory::new(memory_size),
            instructions_executed: 0,
        }
    }

    pub fn load_program(
        &mut self,
        program: &[u8],
    ) -> Result<(), String> {
        if program.is_empty() {
            return Err("Programa vazio".into());
        }

        if program.len() % 4 != 0 {
            return Err(
                "Instrucoes devem ter 4 bytes".into()
            );
        }

        self.memory
            .write(0, program)
            .map_err(str::to_string)?;

        self.cpu = Cpu::new();
        self.instructions_executed = 0;

        Ok(())
    }

    pub fn execute(
        &mut self,
        max_steps: usize,
    ) -> Result<usize, String> {
        let mut completed = 0;

        for _ in 0..max_steps {
            if self.cpu.halted {
                break;
            }

            self.cpu.step(&mut self.memory)?;
            self.instructions_executed += 1;
            completed += 1;
        }

        Ok(completed)
    }

    pub fn register(
        &self,
        index: usize,
    ) -> Option<u64> {
        self.cpu.gpr.get(index).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execute_powerpc_program() {
        let mut engine = ExecutionEngine::new(4096);

        // addi r3, r0, 42
        // addi r4, r3, 8
        let program = [
            0x38, 0x60, 0x00, 0x2A,
            0x38, 0x83, 0x00, 0x08,
        ];

        engine.load_program(&program).unwrap();

        assert_eq!(engine.execute(2).unwrap(), 2);
        assert_eq!(engine.register(3), Some(42));
        assert_eq!(engine.register(4), Some(50));
        assert_eq!(engine.instructions_executed, 2);
    }

    #[test]
    fn reject_invalid_program() {
        let mut engine = ExecutionEngine::new(1024);

        assert!(engine.load_program(&[]).is_err());
        assert!(engine.load_program(&[1, 2, 3]).is_err());
    }
}
