//! Bounded native PowerPC / framebuffer / kernel validation, original demo code only.
use crate::Machine;
pub fn run_demo()->Result<String,String> {
    let mut m=Machine::new();
    // li r3,42 ; addi r4,r3,8 ; stw r4,64(0) ; lwz r5,64(0)
    let code=[0x38,0x60,0,42,0x38,0x83,0,8,0x90,0x80,0,64,0x80,0xA0,0,64];
    m.load_test_program(&code).map_err(str::to_owned)?;
    for _ in 0..4{m.step()?}
    m.gpu.clear([5,20,12,255]);
    m.gpu.rect(20,20,24,24,[85,215,112,255]);
    m.audio.push(&[200,-200,100,-100]).map_err(str::to_owned)?;
    let rgba=m.gpu.framebuffer();
    let green_pixels=rgba.chunks_exact(4).filter(|p|p[1]==215).count();
    Ok(format!("Xenon PPC subset: r3={} r4={} r5={}\nKernel ticks={}\nGPU framebuffer={}x{}, quad={}px\nAudio FIFO={} samples",
        m.cpu.gpr[3],m.cpu.gpr[4],m.cpu.gpr[5],m.kernel.ticks(),m.gpu.width,m.gpu.height,green_pixels,m.audio.queued()))
}
#[cfg(test)]mod tests{#[test]fn demo(){let msg=super::run_demo().unwrap();assert!(msg.contains("r3=42"));assert!(msg.contains("r4=50"));assert!(msg.contains("r5=50"));}}
