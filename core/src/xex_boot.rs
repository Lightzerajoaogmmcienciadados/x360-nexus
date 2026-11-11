//! XEX2 homebrew boot experiment. Only unencrypted, uncompressed, self-contained
//! synthetic XEX2 images are accepted. No Xbox 360 title support.
use crate::execution_engine::ExecutionEngine;
use crate::xex_image::{inspect,StorageKind};
use crate::virtual_memory::PAGE_SIZE;

pub struct BootReport { pub steps:usize,pub pc:u32,pub r3:u64,pub mapped_pages:usize }

pub fn boot_homebrew(data:&[u8])->Result<BootReport,String>{
    let layout=inspect(data)?;
    if layout.encryption_type!=Some(0) || layout.storage!=Some(StorageKind::Uncompressed){
        return Err("XEX requer criptografia ou compressao nao suportada".into());
    }
    let base=layout.image_base.ok_or("Image base ausente")?;
    let entry=layout.entry_point.ok_or("Entry point ausente")?;
    if base as usize%PAGE_SIZE!=0{return Err("Image base desalinhada".into())}
    let payload=data.get(layout.image_offset as usize..).ok_or("Payload ausente")?;
    if payload.is_empty() || payload.len()>1024*1024 || payload.len()%4!=0 {return Err("Payload invalido ou maior que 1 MiB".into())}
    let end=(base as u64)+(payload.len() as u64);
    if (entry as u64)<base as u64 || (entry as u64)+4>end || entry%4!=0 {return Err("Entry point fora do payload".into())}
    // Strictly reject real imports and unsupported XEX metadata: only an original,
    // standalone homebrew demo is eligible for this intentionally limited loader.
    let header=crate::xex_loader::parse(data)?;
    if header.optional_headers.iter().any(|h| h.key==0x000401FF){return Err("Imports Xbox Kernel nao suportados".into())}
    let mut engine=ExecutionEngine::new();
    engine.load_homebrew(base,payload)?;
    engine.pc=entry;
    let steps=engine.run(4096)?;
    Ok(BootReport{steps,pc:engine.pc,r3:engine.registers[3],mapped_pages:engine.memory.mapped_pages()})
}

pub fn demo()->Result<String,String>{
    let mut data=vec![0u8;128];
    data[..4].copy_from_slice(b"XEX2");
    data[8..12].copy_from_slice(&96u32.to_be_bytes());
    data[16..20].copy_from_slice(&64u32.to_be_bytes());
    data[20..24].copy_from_slice(&3u32.to_be_bytes());
    for (i,(key,value)) in [(0x000103FFu32,72u32),(0x00040006,0x10001000),(0x00030000,0x10001000)].iter().enumerate(){
        let pos=24+i*8;data[pos..pos+4].copy_from_slice(&key.to_be_bytes());data[pos+4..pos+8].copy_from_slice(&value.to_be_bytes());
    }
    data[72..76].copy_from_slice(&8u32.to_be_bytes());
    data[76..78].copy_from_slice(&0u16.to_be_bytes());
    data[78..80].copy_from_slice(&1u16.to_be_bytes());
    // addi r3,r0,42; addi r4,r3,8; add r5,r3,r4; private halt sentinel
    data[96..112].copy_from_slice(&[0x38,0x60,0,42,0x38,0x83,0,8,0x7c,0xa3,0x22,0x14,0,0,0,0]);
    // The remaining 16 zero bytes are never executed after the sentinel.
    let report=boot_homebrew(&data)?;
    if report.steps!=4 || report.r3!=42{return Err("Resultado inesperado".into())}
    Ok(format!("XEX Boot Engine 5.0 OK: {} instrucoes, r3={}, PC=0x{:08X}, paginas={}\nDemo XEX2 sintetica sem criptografia. Minecraft nao executavel.",report.steps,report.r3,report.pc,report.mapped_pages))
}
#[cfg(test)]mod tests{use super::*;
#[test]fn boot_demo(){assert!(demo().unwrap().contains("r3=42"));}
#[test]fn reject_invalid(){assert!(boot_homebrew(b"not xex").is_err());}
}
