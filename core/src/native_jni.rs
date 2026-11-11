//! Native JNI entrypoints: conservative XEX metadata and original PCM audio diagnostics.
use jni::{JNIEnv, objects::{JObject, JByteArray, JShortArray}, sys::{jint, jstring}};
use std::sync::{Mutex,OnceLock};
use crate::loader;

fn jstr(mut env:JNIEnv,text:String)->jstring {
    env.new_string(text).map(|s|s.into_raw()).unwrap_or(std::ptr::null_mut())
}
#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeInspectXex(
    mut env:JNIEnv, _:JObject, data:JByteArray)->jstring {
    let msg = match env.convert_byte_array(data) {
        Ok(bytes) => match loader::inspect_xex(&bytes) {
            Ok(info) => format!(
                "XEX2 reconhecido.\nModulo: 0x{:08X}\nCabecalho: {} bytes\nSeguranca: 0x{:X}\nCabecalhos opcionais: {}\n\nDiagnostico somente: nao executa XEX comercial.",
                info.module_flags, info.header_size, info.security_offset, info.optional_headers),
            Err(e)=>format!("XEX: {e}"),
        },
        Err(e)=>format!("Nao foi possivel ler arquivo: {e}"),
    };
    jstr(env,msg)
}
static PHASE:OnceLock<Mutex<f32>>=OnceLock::new();
#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeGenerateAudio(
    mut env:JNIEnv, _:JObject, samples:JShortArray, frequency:jint, sample_rate:jint)->jint {
    if !(80..=1800).contains(&frequency) || !(8000..=192000).contains(&sample_rate) { return -1; }
    let len=match env.get_array_length(&samples) { Ok(n)=>n as usize,Err(_)=>return -1 };
    if len==0 || len>4096 { return -1; }
    let phase=PHASE.get_or_init(||Mutex::new(0.0));
    let Ok(mut phase)=phase.lock() else{return -1};
    let mut buffer=Vec::with_capacity(len);
    let advance=std::f32::consts::TAU*(frequency as f32)/(sample_rate as f32);
    for _ in 0..len {
        // Low amplitude original synth, not the Xbox 360 XMA decoder.
        let v=phase.sin()*0.20 + (phase.sin()*3.0).sin()*0.04;
        buffer.push((v*32767.0).clamp(-32768.0,32767.0) as i16);
        *phase+=advance;
        if *phase>=std::f32::consts::TAU { *phase-=std::f32::consts::TAU; }
    }
    if env.set_short_array_region(&samples,0,&buffer).is_err() { return -1; }
    len as jint
}
