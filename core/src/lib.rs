#![allow(dead_code)]

// X360 Nexus v0.6 - Project Xenon
// CPU PowerPC + Memory + Kernel + GPU Rust JNI

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

use jni::objects::JObject;
use jni::sys::{jint, jintArray, jstring};
use jni::JNIEnv;

pub const GPU_WIDTH: usize = 640;
pub const GPU_HEIGHT: usize = 360;

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
            gpu: Gpu::new(
                GPU_WIDTH,
                GPU_HEIGHT
            ).expect("GPU initialization failed"),
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

        let expected = GPU_WIDTH * GPU_HEIGHT * 4;

        if self.gpu.framebuffer_size() != expected {
            return Err("GPU framebuffer size failed".into());
        }

        let pixel = self.gpu
            .get_pixel(320, 180)
            .map_err(str::to_string)?;

        if pixel != [0, 220, 80, 255] {
            return Err("GPU pixel test failed".into());
        }

        Ok(())
    }

    pub fn self_test() -> Result<String, String> {
        let mut machine = Self::new();

        // PowerPC instructions:
        // addi r3, r0, 42
        // stw r3, 64(r0)
        // lwz r4, 64(r0)

        let program: [u8; 12] = [
            0x38, 0x60, 0x00, 0x2A,
            0x90, 0x60, 0x00, 0x40,
            0x80, 0x80, 0x00, 0x40,
        ];

        machine
            .load_test_program(&program)
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

// Convert Rust RGBA pixels to Android ARGB integers.
pub fn render_gpu_argb() -> Result<Vec<jint>, String> {
    let mut gpu = Gpu::new(
        GPU_WIDTH,
        GPU_HEIGHT
    ).map_err(str::to_string)?;

    gpu.render_test();

    let pixels: Vec<jint> = gpu
        .framebuffer()
        .chunks_exact(4)
        .map(|rgba| {
            let r = rgba[0] as u32;
            let g = rgba[1] as u32;
            let b = rgba[2] as u32;
            let a = rgba[3] as u32;

            ((a << 24)
                | (r << 16)
                | (g << 8)
                | b) as jint
        })
        .collect();

    if pixels.len() != GPU_WIDTH * GPU_HEIGHT {
        return Err("Invalid GPU pixel count".into());
    }

    Ok(pixels)
}

// JNI - CPU and kernel self-test.
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn
Java_com_x360nexus_MainActivity_nativeSelfTest(
    mut env: JNIEnv,
    _object: JObject,
) -> jstring {
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

// JNI - GPU framebuffer for Android Bitmap.
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn
Java_com_x360nexus_MainActivity_nativeGpuFrame(
    env: JNIEnv,
    _object: JObject,
) -> jintArray {
    let result = std::panic::catch_unwind(|| {
        render_gpu_argb()
    });

    let pixels = match result {
        Ok(Ok(value)) => value,
        _ => return std::ptr::null_mut(),
    };

    let length = match i32::try_from(pixels.len()) {
        Ok(value) => value,
        Err(_) => return std::ptr::null_mut(),
    };

    let array = match env.new_int_array(length) {
        Ok(value) => value,
        Err(_) => return std::ptr::null_mut(),
    };

    if env.set_int_array_region(
        &array,
        0,
        &pixels
    ).is_err() {
        return std::ptr::null_mut();
    }

    array.into_raw()
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

    #[test]
    fn gpu_argb_test() {
        let pixels = render_gpu_argb().unwrap();

        assert_eq!(
            pixels.len(),
            GPU_WIDTH * GPU_HEIGHT
        );

        let center = pixels[
            180 * GPU_WIDTH + 320
        ] as u32;

        assert_eq!(center, 0xFF00DC50);
    }
}
