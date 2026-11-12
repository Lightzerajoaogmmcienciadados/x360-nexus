//! X360 Nexus Native: real Android/Rust components; experimental emulation only.
//! Retail Xbox 360 games are NOT executable with this project.
pub mod audio;
pub mod cpu;
pub mod gpu;
pub mod xenos_lab;
pub mod xenos_pipeline;
mod xenos_lab_jni;
pub mod input;
pub mod kernel;
pub mod loader;
pub mod memory;
pub mod galaxy;
mod galaxy_jni;
mod native_jni;
mod compat_lab;
use audio::Audio;
use cpu::Cpu;
use gpu::Gpu;
use input::Controller;
use kernel::Kernel;
use memory::Memory;

pub struct Machine {
    pub cpu:Cpu,
    pub memory:Memory,
    pub kernel:Kernel,
    pub audio:Audio,
    pub gpu:Gpu,
    pub controller:Controller,
}
impl Machine {
    pub fn new()->Self {
        Self { cpu:Cpu::new(), memory:Memory::new(1024*1024),kernel:Kernel::new(),
            audio:Audio::new(48000,48000),gpu:Gpu::new(640,360).expect("valid dimensions"),
            controller:Controller::default() }
    }
    pub fn load_test_program(&mut self,program:&[u8])->Result<(),&'static str>{
        self.memory.write(0,program)?;
        self.cpu=Cpu::new();
        Ok(())
    }
    pub fn step(&mut self)->Result<(),String> {
        self.cpu.step(&mut self.memory)?;
        self.kernel.tick();
        Ok(())
    }
}
impl Default for Machine{fn default()->Self{Self::new()}}
#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeSelfTest(
    mut env:jni::JNIEnv,_:jni::objects::JObject)->jni::sys::jstring {
    let msg=match galaxy::run_demo(){
      Ok(v)=>format!("Rust/PowerPC: OK\n{v}\nXenos: somente diagnostico\nJogos comerciais: nao suportados"),
      Err(e)=>format!("Falha no nucleo: {e}")
    };
    env.new_string(msg).map(|s|s.into_raw()).unwrap_or(std::ptr::null_mut())
}
#[cfg(test)]
mod tests{use super::*;#[test]fn machine(){
    let mut m=Machine::new();
    m.load_test_program(&[0x38,0x60,0,42]).unwrap();
    m.step().unwrap();
    assert_eq!(m.cpu.gpr[3],42);
}}
