
#![allow(dead_code)]

pub mod audio;
pub mod cpu;
pub mod gpu;
pub mod gpu3d;
pub mod gpu_advanced;
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
use jni::sys::{jfloat, jint, jintArray, jlong, jstring};
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
            gpu: Gpu::new(GPU_WIDTH, GPU_HEIGHT)
                .expect("GPU initialization failed"),
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

        if self.gpu.framebuffer_size()
            != GPU_WIDTH * GPU_HEIGHT * 4
        {
            return Err("GPU framebuffer failed".into());
        }

        if self.gpu.get_pixel(320, 180)
            .map_err(str::to_string)?
            != [0, 220, 80, 255]
        {
            return Err("GPU pixel test failed".into());
        }

        Ok(())
    }

    pub fn self_test() -> Result<String, String> {
        let mut machine = Self::new();

        // PowerPC: addi, stw, lwz
        let program: [u8; 12] = [
            0x38, 0x60, 0x00, 0x2A,
            0x90, 0x60, 0x00, 0x40,
            0x80, 0x80, 0x00, 0x40,
        ];

        machine.load_test_program(&program)
            .map_err(str::to_string)?;
        machine.run_steps(3)?;

        if machine.cpu.gpr[3] != 42
            || machine.cpu.gpr[4] != 42
        {
            return Err("PowerPC test failed".into());
        }

        if machine.kernel.ticks() != 3 {
            return Err("Kernel tick failed".into());
        }

        machine.gpu_test()?;

        let cube = gpu3d::render_cube_frame(
            GPU_WIDTH, GPU_HEIGHT, 0.4, 0.3
        ).map_err(str::to_string)?;

        if cube.len() != GPU_WIDTH * GPU_HEIGHT * 4 {
            return Err("GPU 3D framebuffer failed".into());
        }

        let advanced =
            gpu_advanced::render_advanced_frame(
                GPU_WIDTH, GPU_HEIGHT, 0.4, 0.3
            ).map_err(str::to_string)?;

        if advanced.len()
            != GPU_WIDTH * GPU_HEIGHT * 4
        {
            return Err("GPU advanced framebuffer failed".into());
        }

        Ok(format!(
            "X360 NEXUS v0.8 - PROJECT XENON\n\
             Rust JNI: OK\n\
             CPU PowerPC: OK\n\
             Memoria LWZ/STW: OK\n\
             Registrador r4: {}\n\
             Kernel: {} ticks\n\
             GPU 2D: OK\n\
             GPU 3D: OK\n\
             GPU avancada: OK\n\
             Z-buffer: disponivel\n\
             Resolucao: {}x{}\n\
             Texturas: procedurais\n\
             Iluminacao: experimental\n\
             Controles: Android virtual\n\
             Audio: experimental\n\
             Xenos real: nao implementado\n\
             Jogos comerciais: nao suportados",
            machine.cpu.gpr[4],
            machine.kernel.ticks(),
            GPU_WIDTH,
            GPU_HEIGHT
        ))
    }
}

impl Default for Machine {
    fn default() -> Self {
        Self::new()
    }
}

fn rgba_to_argb(rgba: &[u8]) -> Vec<jint> {
    rgba.chunks_exact(4)
        .map(|p| {
            let r = p[0] as u32;
            let g = p[1] as u32;
            let b = p[2] as u32;
            let a = p[3] as u32;

            ((a << 24)
                | (r << 16)
                | (g << 8)
                | b) as jint
        })
        .collect()
}

fn make_java_array(
    env: &JNIEnv,
    pixels: &[jint],
) -> jintArray {
    let len = match i32::try_from(pixels.len()) {
        Ok(n) => n,
        Err(_) => return std::ptr::null_mut(),
    };

    let array = match env.new_int_array(len) {
        Ok(a) => a,
        Err(_) => return std::ptr::null_mut(),
    };

    if env.set_int_array_region(&array, 0, pixels).is_err() {
        return std::ptr::null_mut();
    }

    array.into_raw()
}

pub fn render_gpu_argb() -> Result<Vec<jint>, String> {
    let mut gpu = Gpu::new(GPU_WIDTH, GPU_HEIGHT)
        .map_err(str::to_string)?;
    gpu.render_test();
    Ok(rgba_to_argb(gpu.framebuffer()))
}

pub fn render_dynamic_argb(
    x: usize,
    y: usize,
    tick: u64,
) -> Result<Vec<jint>, String> {
    let mut gpu = Gpu::new(GPU_WIDTH, GPU_HEIGHT)
        .map_err(str::to_string)?;
    gpu.render_scene(x, y, tick);
    Ok(rgba_to_argb(gpu.framebuffer()))
}

pub fn render_3d_argb(
    yaw: f32,
    pitch: f32,
) -> Result<Vec<jint>, String> {
    if !yaw.is_finite() || !pitch.is_finite() {
        return Err("Invalid camera angles".into());
    }

    let rgba = gpu3d::render_cube_frame(
        GPU_WIDTH, GPU_HEIGHT, yaw, pitch
    ).map_err(str::to_string)?;

    Ok(rgba_to_argb(&rgba))
}

pub fn render_advanced_argb(
    yaw: f32,
    pitch: f32,
) -> Result<Vec<jint>, String> {
    if !yaw.is_finite() || !pitch.is_finite() {
        return Err("Invalid camera angles".into());
    }

    let rgba = gpu_advanced::render_advanced_frame(
        GPU_WIDTH, GPU_HEIGHT, yaw, pitch
    ).map_err(str::to_string)?;

    Ok(rgba_to_argb(&rgba))
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn
Java_com_x360nexus_MainActivity_nativeSelfTest(
    env: JNIEnv,
    _object: JObject,
) -> jstring {
    let result = std::panic::catch_unwind(|| {
        Machine::self_test()
    });

    let message = match result {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => format!("Falha: {}", e),
        Err(_) => "Falha interna Rust".to_string(),
    };

    match env.new_string(message) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

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

    match result {
        Ok(Ok(pixels)) => make_java_array(&env, &pixels),
        _ => std::ptr::null_mut(),
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn
Java_com_x360nexus_MainActivity_nativeGpuFrameDynamic(
    env: JNIEnv,
    _object: JObject,
    x: jint,
    y: jint,
    tick: jlong,
) -> jintArray {
    let result = std::panic::catch_unwind(|| {
        render_dynamic_argb(
            x.max(0) as usize,
            y.max(0) as usize,
            tick.max(0) as u64,
        )
    });

    match result {
        Ok(Ok(pixels)) => make_java_array(&env, &pixels),
        _ => std::ptr::null_mut(),
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn
Java_com_x360nexus_MainActivity_nativeGpu3DFrame(
    env: JNIEnv,
    _object: JObject,
    yaw: jfloat,
    pitch: jfloat,
) -> jintArray {
    let result = std::panic::catch_unwind(|| {
        render_3d_argb(yaw, pitch)
    });

    match result {
        Ok(Ok(pixels)) => make_java_array(&env, &pixels),
        _ => std::ptr::null_mut(),
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn
Java_com_x360nexus_MainActivity_nativeGpuAdvancedFrame(
    env: JNIEnv,
    _object: JObject,
    yaw: jfloat,
    pitch: jfloat,
) -> jintArray {
    let result = std::panic::catch_unwind(|| {
        render_advanced_argb(yaw, pitch)
    });

    match result {
        Ok(Ok(pixels)) => make_java_array(&env, &pixels),
        _ => std::ptr::null_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn machine_test() {
        let s = Machine::self_test().unwrap();
        assert!(s.contains("PowerPC: OK"));
        assert!(s.contains("GPU avancada: OK"));
    }

    #[test]
    fn gpu_2d_test() {
        let pixels = render_gpu_argb().unwrap();

        assert_eq!(
            pixels.len(),
            GPU_WIDTH * GPU_HEIGHT
        );

        assert_eq!(
            pixels[180 * GPU_WIDTH + 320] as u32,
            0xFF00DC50
        );
    }

    #[test]
    fn gpu_3d_test() {
        let pixels = render_3d_argb(0.4, 0.3).unwrap();

        assert_eq!(
            pixels.len(),
            GPU_WIDTH * GPU_HEIGHT
        );

        assert!(pixels.iter().any(
            |p| (*p as u32) != 0xFF060D0C
        ));
    }

    #[test]
    fn gpu_advanced_test() {
        let pixels = render_advanced_argb(
            0.4, 0.3
        ).unwrap();

        assert_eq!(
            pixels.len(),
            GPU_WIDTH * GPU_HEIGHT
        );

        assert!(pixels.iter().any(
            |p| (*p as u32) != 0xFF060D0C
        ));
    }
}
