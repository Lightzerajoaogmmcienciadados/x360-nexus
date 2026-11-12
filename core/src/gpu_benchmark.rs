//! X360 Nexus v10.0 experimental GPU workload and frame pacing diagnostics.
//! Synthetic geometry only: no actual Xbox 360 Xenos packet or shader execution.
use crate::xenos_pipeline::{Pipeline,Vertex};
use std::time::Instant;
use jni::{JNIEnv,objects::JObject,sys::jstring};
#[derive(Debug,Clone)]
pub struct Benchmark {
    pub frames:u32,
    pub draw_calls:u64,
    pub triangles:u64,
    pub pixels:u64,
    pub depth_rejections:u64,
    pub texture_samples:u64,
    pub elapsed_ms:u128,
    pub checksum:u64,
}
pub fn run(frames:u32)->Result<Benchmark,String>{
    if !(1..=120).contains(&frames){return Err("Frame budget 1..120".into())}
    let mut gpu=Pipeline::new(320,180)?;
    let start=Instant::now();
    let mut result=Benchmark{frames,draw_calls:0,triangles:0,pixels:0,
        depth_rejections:0,texture_samples:0,elapsed_ms:0,checksum:0};
    for frame in 0..frames {
        gpu.clear(0xff06160d);
        for n in 0..24u32 {
            let x=12.0+(n%8) as f32*38.0;
            let y=10.0+(n/8) as f32*55.0;
            let shift=((frame+n)%7) as f32;
            let z=0.2+(n%4) as f32*0.12;
            let tint=n as f32/24.0;
            let verts=[
                Vertex::new(x+shift,y,z,[0.1+tint,0.9,0.3],[0.0,0.0]),
                Vertex::new(x+32.0,y+shift,z,[0.2,0.7,1.0],[1.0,0.0]),
                Vertex::new(x+shift+8.0,y+39.0,z,[1.0,0.4,0.2],[0.5,1.0]),
            ];
            gpu.draw_indexed(&verts,&[0,1,2],true)?;
        }
        let stats=gpu.stats();
        result.draw_calls+=stats.draw_calls as u64;
        result.triangles+=stats.indexed_triangles as u64;
        result.pixels+=stats.shaded_pixels;
        result.depth_rejections+=stats.depth_rejections;
        result.texture_samples+=stats.bilinear_samples;
        result.checksum=result.checksum.wrapping_add(
            gpu.pixels().iter().step_by(97).map(|p|*p as u64).sum::<u64>());
    }
    result.elapsed_ms=start.elapsed().as_millis();
    Ok(result)
}
pub fn report(frames:u32)->Result<String,String>{
    let b=run(frames)?;
    let avg=b.elapsed_ms as f64/b.frames as f64;
    Ok(format!(
      "XENOS LAB v10.0 - GPU SOFTWARE\nFrames: {}\nTempo total: {} ms\nMedia: {:.2} ms/frame\nDraw calls: {}\nTriangulos: {}\nPixels: {}\nZ-test rejeitados: {}\nAmostras bilineares: {}\nChecksum: {:016X}\n\nCenas originais de teste; NAO e Xenos real nem executa jogos comerciais.",
      b.frames,b.elapsed_ms,avg,b.draw_calls,b.triangles,b.pixels,
      b.depth_rejections,b.texture_samples,b.checksum))
}
#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeGpuBenchmark(
    mut env:JNIEnv,_:JObject,frames:jni::sys::jint)->jstring{
    let msg=report(frames.max(0) as u32).unwrap_or_else(|err|format!("GPU: {err}"));
    env.new_string(msg).map(|s|s.into_raw()).unwrap_or(std::ptr::null_mut())
}
#[cfg(test)]mod tests{
    #[test]fn frames_have_consistent_counts(){
        let b=super::run(4).unwrap();
        assert_eq!(b.frames,4);assert_eq!(b.draw_calls,96);
        assert_eq!(b.triangles,96);assert!(b.pixels>0);
        assert!(b.texture_samples>0);
    }
    #[test]fn bounded_frames(){assert!(super::run(121).is_err());}
}
