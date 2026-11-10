//! X360 Nexus experimental emulator infrastructure.
//! This is NOT a functioning Xbox 360 emulator.
pub mod audio;
pub mod cpu;
pub mod gpu;
pub mod input;
pub mod kernel;
pub mod loader;
pub mod memory;

use audio::Audio;use cpu::Cpu;use gpu::Gpu;use input::Controller;use kernel::Kernel;use memory::Memory;
pub struct Machine {pub cpu:Cpu,pub memory:Memory,pub kernel:Kernel,pub audio:Audio,pub gpu:Gpu,pub controller:Controller}
impl Machine {
 pub fn new()->Self{Self{cpu:Cpu::new(),memory:Memory::new(1024*1024),kernel:Kernel::new(),audio:Audio::new(48000,48000),gpu:Gpu::new(640,360).unwrap(),controller:Controller::default()}}
 pub fn load_test_program(&mut self,program:&[u8])->Result<(),&'static str>{self.memory.write(0,program)?;self.cpu.pc=0;Ok(())}
 pub fn step(&mut self)->Result<(),String>{self.cpu.step(&self.memory)?;self.kernel.tick();Ok(())}
}
impl Default for Machine{fn default()->Self{Self::new()}}
#[cfg(test)]mod tests{use super::*;#[test]fn machine(){let mut m=Machine::new();m.load_test_program(&[0x38,0x60,0,42]).unwrap();m.step().unwrap();assert_eq!(m.cpu.gpr[3],42);assert_eq!(m.kernel.ticks(),1);}}


// Android JNI: runs a fixed, safe PowerPC test program, not a game.
#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeSelfTest(
    mut env: jni::JNIEnv,
    _object: jni::objects::JObject,
) -> jni::sys::jstring {
    let result = std::panic::catch_unwind(|| {
        let mut machine = Machine::new();
        machine.load_test_program(&[0x38, 0x60, 0x00, 0x2a])
            .map_err(|e| e.to_string())?;
        machine.step()?;
        if machine.cpu.gpr[3] != 42 || machine.kernel.ticks() != 1 {
            return Err("resultado inesperado".to_string());
        }
        Ok::<String, String>(format!(
            "Rust integrado: OK\nCPU PowerPC: addi r3, r0, 42 = {}\nKernel experimental: {} tick\nGPU/Audio: estruturas de teste\nJogos comerciais: nao suportados",
            machine.cpu.gpr[3], machine.kernel.ticks()
        ))
    });
    let message = match result {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => format!("Falha no teste: {error}"),
        Err(_) => "Falha interna no nucleo Rust".to_string(),
    };
    match env.new_string(message) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}
