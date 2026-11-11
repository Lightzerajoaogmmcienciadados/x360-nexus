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
            Ok(info) => match loader::inspect_xex_optionals(&bytes) {
                Ok(items) => format!(
                    "XEX2 reconhecido.\nFlags modulo: 0x{:08X}\nOffset dados PE: 0x{:X}\nInfo seguranca: 0x{:X}\nCabecalhos opcionais: {}\n\n{}\n\nDiagnostico apenas: imagem XEX comercial nao executavel.",
                    info.module_flags, info.header_size, info.security_offset, info.optional_headers,
                    items.iter().take(32)
                        .map(|item| format!("0x{:08X} -> 0x{:08X}",item.key,item.value))
                        .collect::<Vec<_>>().join("\n")
                ),
                Err(e) => format!("XEX2 reconhecido, tabela incompleta: {e}"),
            },
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


/// Run raw big-endian PowerPC instructions in a small, isolated *diagnostic*
/// machine, with an instruction budget. Not an XEX boot or Xbox 360 runtime.
pub fn execute_raw_powerpc(program: &[u8], max_steps: usize) -> Result<String, String> {
    if program.is_empty() || program.len() > 64 * 1024 || program.len() % 4 != 0 {
        return Err("Programa PPC deve ter 4 a 65536 bytes, alinhados a 4".into());
    }
    if max_steps == 0 || max_steps > 20_000 {
        return Err("Orcamento de instrucoes invalido".into());
    }
    let mut machine = crate::Machine::new();
    machine.load_test_program(program).map_err(str::to_string)?;
    let mut executed = 0;
    while executed < max_steps {
        if machine.cpu.pc == program.len() { break; }
        if machine.cpu.pc > program.len() {
            return Err(format!("Salto alem do codigo em 0x{:X}", machine.cpu.pc));
        }
        machine.step()?;
        executed += 1;
    }
    if machine.cpu.pc != program.len() {
        return Err(format!("Limite de {max_steps} instrucoes atingido (PC=0x{:X})",machine.cpu.pc));
    }
    Ok(format!(
        "Programa PPC experimental concluido\nInstrucoes: {}\nPC: 0x{:X}\nr3={} r4={} r5={}\nKernel ticks={}\nSem APIs de jogos Xbox 360",
        executed, machine.cpu.pc, machine.cpu.gpr[3], machine.cpu.gpr[4],
        machine.cpu.gpr[5], machine.kernel.ticks(),
    ))
}

#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeExecuteRawPpc(
    mut env: JNIEnv, _: JObject, data: JByteArray
) -> jstring {
    let msg = match env.convert_byte_array(data) {
        Ok(program) => execute_raw_powerpc(&program, 20_000)
            .unwrap_or_else(|e| format!("Erro no programa PowerPC: {e}")),
        Err(err) => format!("Arquivo PPC invalido: {err}"),
    };
    jstr(env, msg)
}

#[cfg(test)]
mod raw_ppc_tests {
    use super::*;
    #[test] fn executes_original_raw_ppc_instructions() {
        let bytes = [0x38,0x60,0,42, 0x38,0x83,0,8];
        let result = execute_raw_powerpc(&bytes,100).unwrap();
        assert!(result.contains("r3=42"));
        assert!(result.contains("r4=50"));
    }
    #[test] fn infinite_branch_respects_instruction_budget() {
        let loop_bytes=[0x48,0,0,0];
        assert!(execute_raw_powerpc(&loop_bytes,5).unwrap_err().contains("Limite"));
    }
    #[test] fn refuses_unaligned_program() {
        assert!(execute_raw_powerpc(&[1,2,3],20).is_err());
    }
}
