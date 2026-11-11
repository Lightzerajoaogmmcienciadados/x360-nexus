
#![allow(dead_code)]

// X360 Nexus v0.6 - Project Xenon
// Experimental Rust emulator infrastructure.

pub mod audio;
pub mod cpu;
pub mod gpu;
pub mod input;
pub mod kernel;
pub mod loader;
pub mod memory;

use audio::Audio;
use cpu::Cpu;
use gpu::Gpu;
use input::Controller;
use kernel::Kernel;
use memory::Memory;

pub struct Machine {
    pub cpu: Cpu,
    pub memory: Memory,
    pub kernel: Kernel,
    pub audio: Audio,
    pub gpu: Gpu,
    pub controller: Controller,
}

impl Machine {
    pub fn new() -> Self {
        Self {
            cpu: Cpu::new(),
            memory: Memory::new(1024 * 1024),
            kernel: Kernel::new(),
            audio: Audio::new(48000, 48000),
            gpu: Gpu::new(640, 360).unwrap(),
            controller: Controller::default(),
        }
    }

    pub fn load_test_program(
        &mut self,
        program: &[u8],
    ) -> Result<(), &'static str> {
        self.memory.write(0, program)?;
        self.cpu.pc = 0;
        self.cpu.halted = false;
        Ok(())
    }

    pub fn step(&mut self) -> Result<(), String> {
        self.cpu.step(&mut self.memory)?;
        self.kernel.tick();
        Ok(())
    }

    pub fn run_steps(
        &mut self,
        limit: usize,
    ) -> Result<usize, String> {
        let mut executed = 0;

        for _ in 0..limit {
            self.step()?;
            executed += 1;

            if self.cpu.halted {
                break;
            }
        }

        Ok(executed)
    }

    pub fn gpu_test(&mut self) -> Result<(), String> {
        self.gpu.render_test();

        if self.gpu.frame_count() != 1 {
            return Err("GPU frame test failed".into());
        }

        let expected = 640 * 360 * 4;

        if self.gpu.framebuffer_size() != expected {
            return Err("GPU framebuffer failed".into());
        }

        let pixel = self.gpu.get_pixel(320, 180)
            .map_err(str::to_string)?;

        if pixel != [0, 220, 80, 255] {
            return Err("GPU pixel test failed".into());
        }

        Ok(())
    }

    pub fn self_test() -> Result<String, String> {
        let mut machine = Machine::new();

        // addi r3, r0, 42
        // stw r3, 64(r0)
        // lwz r4, 64(r0)
        let program: [u8; 12] = [
            0x38, 0x60, 0x00, 0x2A,
            0x90, 0x60, 0x00, 0x40,
            0x80, 0x80, 0x00, 0x40,
        ];

        machine.load_test_program(&program)
            .map_err(str::to_string)?;

        machine.run_steps(3)?;

        if machine.cpu.gpr[3] != 42 {
            return Err("CPU ADDI failed".into());
        }

        if machine.cpu.gpr[4] != 42 {
            return Err("Memory LWZ/STW failed".into());
        }

        if machine.kernel.ticks() != 3 {
            return Err("Kernel tick failed".into());
        }

        machine.gpu_test()?;

        Ok(format!(
            "X360 NEXUS v0.6 - PROJECT XENON\n\
             Rust JNI: OK\n\
             CPU PowerPC: OK\n\
             Memoria LWZ/STW: OK\n\
             Registrador r4: {}\n\
             Kernel: {} ticks\n\
             GPU framebuffer: OK\n\
             GPU resolucao: {}x{}\n\
             GPU frames: {}\n\
             GPU renderizacao: OK\n\
             Audio: experimental\n\
             Controles: estrutura inicial\n\
             Jogos comerciais: nao suportados",
            machine.cpu.gpr[4],
            machine.kernel.ticks(),
            machine.gpu.width,
            machine.gpu.height,
            machine.gpu.frame_count()
        ))
    }
}

impl Default for Machine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn machine_test() {
        let result = Machine::self_test().unwrap();

        assert!(result.contains("PowerPC: OK"));
        assert!(result.contains("LWZ/STW: OK"));
        assert!(result.contains("GPU framebuffer: OK"));
        assert!(result.contains("Kernel: 3 ticks"));
    }
}

// Android JNI
// Preserve nativeSelfTest from the existing app.

#[no_mangle]
pub extern "system" fn
Java_com_x360nexus_MainActivity_nativeSelfTest(
    mut env: jni::JNIEnv,
    _object: jni::objects::JObject,
) -> jni::sys::jstring {
    let result = std::panic::catch_unwind(|| {
        Machine::self_test()
    });

    let message = match result {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => {
            format!("Falha no teste: {}", error)
        }
        Err(_) => {
            "Falha interna no nucleo Rust".to_string()
        }
    };

    match env.new_string(message) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}
