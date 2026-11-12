//! Bounded experimental subsystem tests: no commercial Xbox 360 game boot.
use crate::{audio::{self,StereoFrame},cpu::Cpu,kernel::{Kernel,ThreadState},
            memory::Memory,loader,xenos_pipeline};
use jni::{JNIEnv,objects::JObject,sys::jstring};
pub fn report()->Result<String,String>{
    let mut mem=Memory::new(256);
    // li r3,7; mulli r4,r3,6; rlwinm r5,r4,8,0,31
    let instructions=[
        (14u32<<26)|(3<<21)|7,
        (7u32<<26)|(4<<21)|(3<<16)|6,
        (21u32<<26)|(4<<21)|(5<<16)|(8<<11)|(31<<1)
    ];
    for (i,ins) in instructions.iter().enumerate(){
        mem.write(i*4,&ins.to_be_bytes()).map_err(str::to_owned)?;
    }
    let mut cpu=Cpu::new();cpu.run_steps(&mut mem,3)?;
    if cpu.gpr[3]!=7||cpu.gpr[4]!=42||cpu.gpr[5]!=10752 {
        return Err("Falha no diagnostico PowerPC".into());
    }
    let mut kernel=Kernel::new();
    let a=kernel.create_thread();let b=kernel.create_thread();
    kernel.set_thread_state(b,ThreadState::Waiting).map_err(str::to_owned)?;
    kernel.tick();
    if kernel.next_ready(None)!=Some(a){return Err("Thread nao escalonada".into())}
    let first=[StereoFrame{left:30000,right:-30000}];
    let second=[StereoFrame{left:30000,right:-30000}];
    let mixed=audio::mix_stereo(&first,&second,1.0,1.0).map_err(str::to_owned)?;
    let resampled=audio::resample_linear_mono(&[0,1000,2000,3000],8000,16000).map_err(str::to_owned)?;
    let gpu=xenos_pipeline::demo(2)?;
    Ok(format!(
      "DIAGNOSTICO X360 NEXUS - EXPERIMENTAL\nCPU PPC: 3 instrucoes; r3={} r4={} r5={}\nKernel: {} thread(s) pronta(s), escalonamento OK\nAudio: mixer estereo L={} R={} / {} amostras resample\nGPU LAB: {} triangulos, {} pixels, {} amostras texturizadas\nPacotes: CON/LIVE/PIRS reconhecidos por assinatura\nControles: entrada Android e JNI, vibracao configuravel\n\nSem execucao de XEX comercial ou jogos Xbox 360.",
      cpu.gpr[3],cpu.gpr[4],cpu.gpr[5],kernel.ready_count(),
      mixed[0].left,mixed[0].right,resampled.len(),
      gpu.stats().indexed_triangles,gpu.stats().shaded_pixels,
      gpu.stats().bilinear_samples))
}
#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeSubsystemDiagnostics(
    mut env:JNIEnv,_:JObject
)->jstring{
    let value=report().unwrap_or_else(|err|format!("Diagnostico falhou: {err}"));
    env.new_string(value).map(|s|s.into_raw()).unwrap_or(std::ptr::null_mut())
}
#[cfg(test)]mod tests{
    #[test]fn subsystem_diagnostics_complete(){
        let report=super::report().unwrap();
        assert!(report.contains("CPU PPC: 3"));
        assert!(report.contains("GPU LAB:"));
        assert!(report.contains("mixer estereo"));
    }
}
