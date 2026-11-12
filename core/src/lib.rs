//! Project Xenon 4.3 Galaxy: diagnostic PPC & XEX2 loader; not a commercial game emulator.
pub mod audio;
pub mod cpu;
pub mod gpu;
pub mod input;
pub mod kernel;
pub mod loader;
pub mod memory;
pub mod galaxy;
pub mod xex_trace;
pub mod runtime;
pub mod scheduler;
pub mod mmu;
mod galaxy_jni;
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
            cpu: Cpu::new(), memory: Memory::new(1024*1024), kernel: Kernel::new(),
            audio: Audio::new(48000,48000), gpu: Gpu::new(640,360).unwrap(),
            controller: Controller::default(),
        }
    }
    pub fn load_test_program(&mut self, program: &[u8]) -> Result<(), &'static str> {
        self.memory.write(0,program)?;
        self.cpu.pc=0;
        Ok(())
    }
    pub fn step(&mut self) -> Result<(), String> {
        self.cpu.step_mut(&mut self.memory)?;
        self.kernel.tick();
        Ok(())
    }
}
impl Default for Machine {fn default()->Self{Self::new()}}

#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeSelfTest(
    mut env: jni::JNIEnv, _: jni::objects::JObject
) -> jni::sys::jstring {
    let msg=match galaxy::run_demo() {
        Ok(v)=>format!("Rust integrado: OK\n{v}\nGPU/Audio: infraestrutura experimental\nJogos comerciais: nao suportados"),
        Err(e)=>format!("Erro no nucleo: {e}"),
    };
    env.new_string(msg).map(|s|s.into_raw()).unwrap_or(std::ptr::null_mut())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn machine() {
        let mut m=Machine::new();
        m.load_test_program(&[0x38,0x60,0,42]).unwrap();
        m.step().unwrap();
        assert_eq!(m.cpu.gpr[3],42);
    }
}