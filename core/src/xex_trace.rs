//! Bounded diagnostic execution of a small executable slice copied by the
//! C++ XEX2 loader. This is NOT an Xbox 360 kernel or a commercial-game runner.
use crate::{cpu::Cpu, memory::Memory};
use jni::{objects::{JByteArray, JObject}, sys::{jint, jlong, jstring}, JNIEnv};
use std::fmt::Write;

fn short_disasm(word: u32) -> String {
    let op = word >> 26;
    let rd=(word>>21)&31;
    let ra=(word>>16)&31;
    let rb=(word>>11)&31;
    let imm=(word as u16 as i16) as i32;
    match op {
        14 => format!("addi r{rd},r{ra},{imm}"),
        15 => format!("addis r{rd},r{ra},{imm}"),
        24 => if word==0x6000_0000 {"nop".into()} else {format!("ori r{ra},r{rd},0x{:X}",word&0xFFFF)},
        25 => format!("oris r{ra},r{rd},0x{:X}",word&0xFFFF),
        18 => "b/bl (desvio)".into(),
        17 => "sc (pedido de syscall)".into(),
        31 if ((word>>1)&1023)==266 => format!("add r{rd},r{ra},r{rb}"),
        _ => format!("opcode {op} (nao implementado)"),
    }
}

pub fn trace_window(code: &[u8], guest_base: u32, max_steps: usize) -> String {
    let mut out=String::from("TRACE PPC64 — somente janela de codigo\n");
    if code.len()<4 || code.len()>4096 || code.len()%4!=0 || max_steps==0 {
        return "Janela de instrucoes invalida".into();
    }
    let mut mem=Memory::new(code.len());
    if let Err(err)=mem.write(0,code){return format!("Memoria: {err}");}
    let mut cpu=Cpu::new();
    let max_steps=max_steps.min(32);
    let mut executed=0;
    for _ in 0..max_steps {
        let pc=cpu.pc;
        if pc%4!=0 || pc.checked_add(4).is_none_or(|end|end>code.len()) {
            let _=writeln!(out,"PC fora da janela: 0x{:X}",guest_base as u64 + pc as u64);
            break;
        }
        let word=match mem.read_u32_be(pc){Ok(v)=>v,Err(e)=>{let _=writeln!(out,"Fault: {e}");break;}};
        let instr=short_disasm(word);
        let _=writeln!(out,"PC 0x{:08X}: {:08X} | {}",guest_base as u64 + pc as u64,word,instr);
        if word==0x4400_0002 || (word>>26)==17 {
            let _=writeln!(out,"PARADA: syscall sc, r0={} — xboxkrnl/xam HLE nao implementado",cpu.gpr[0]);
            break;
        }
        match cpu.step(&mem) {
            Ok(())=> executed+=1,
            Err(err)=>{let _=writeln!(out,"PARADA: {err}");break;}
        }
    }
    let _=writeln!(out,"\nExecutadas: {executed}\nPC: 0x{:08X}\nLR: 0x{:016X} CTR: 0x{:016X}",guest_base as u64 + cpu.pc as u64,cpu.lr,cpu.ctr);
    for i in 0..8 {let _=writeln!(out,"r{i}=0x{:016X}",cpu.gpr[i]);}
    out
}

#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeTracePpc(
    mut env: JNIEnv, _: JObject, code: JByteArray, guest_pc: jlong, steps: jint,
) -> jstring {
    let result = match env.convert_byte_array(&code) {
        Ok(data) if (0..=u32::MAX as i64).contains(&guest_pc) =>
            trace_window(&data, guest_pc as u32, steps.clamp(1,32) as usize),
        Ok(_) => "Endereco da CPU fora de 32 bits".to_string(),
        Err(e) => format!("Falha JNI: {e}"),
    };
    env.new_string(result).map(|s|s.into_raw()).unwrap_or(std::ptr::null_mut())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sum_three_instructions() {
        let code=[0x38,0x60,0,5,0x38,0x80,0,7,0x7c,0xa3,0x22,0x14,0x60,0,0,0];
        let output=trace_window(&code,0x82001000,4);
        assert!(output.contains("Executadas: 4"),"{output}");
        assert!(output.contains("r5=0x000000000000000C"),"{output}");
    }
    #[test]
    fn stop_unknown() {
        let output=trace_window(&[0xFF,0xFF,0xFF,0xFF],0x82000000,1);
        assert!(output.contains("PARADA:"));
    }
    #[test]
    fn stop_syscall() {
        let output=trace_window(&[0x44,0x00,0x00,0x02],0x82000000,1);
        assert!(output.contains("syscall sc"));
    }
}