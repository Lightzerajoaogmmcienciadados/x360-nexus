//! Bounded inspection of real XEX2 files. No decryption, import resolution or commercial boot.
use crate::{xex_image::{inspect,StorageKind},xex_loader::parse};

pub const MAX_XEX_BYTES:usize=32*1024*1024;
#[derive(Debug,PartialEq,Eq)]
pub enum LoadStatus { CandidateHomebrew, Unsupported(String) }
#[derive(Debug)]
pub struct RealXexReport {
    pub image_base:Option<u32>,pub entry_point:Option<u32>,pub optional_count:usize,
    pub payload_size:usize,pub import_descriptor:bool,pub status:LoadStatus,
}
pub fn analyze(data:&[u8])->Result<RealXexReport,String>{
    if data.len()>MAX_XEX_BYTES{return Err("XEX acima do limite de 32 MiB".into())}
    let header=parse(data)?;
    let image=inspect(data)?;
    let imports=header.optional_headers.iter().any(|h|h.key==0x000401FF);
    let mut blockers=Vec::new();
    if image.storage!=Some(StorageKind::Uncompressed){blockers.push("armazenamento nao suportado")}
    if image.encryption_type!=Some(0){blockers.push("criptografia nao suportada")}
    if imports{blockers.push("imports Xbox 360 nao resolvidos")}
    if image.image_base.is_none() || image.entry_point.is_none(){blockers.push("endereco base/entrada ausente")}
    if let (Some(base),Some(entry))=(image.image_base,image.entry_point){
        if entry<base || (entry as u64)>=base as u64+image.payload_bytes as u64 || entry%4!=0 {blockers.push("entry point fora do payload plano")}
    }
    if image.payload_bytes==0 || image.payload_bytes>1024*1024 {blockers.push("payload vazio ou acima do limite de homebrew")}
    let status=if blockers.is_empty(){LoadStatus::CandidateHomebrew}else{LoadStatus::Unsupported(blockers.join("; "))};
    Ok(RealXexReport{image_base:image.image_base,entry_point:image.entry_point,optional_count:header.optional_headers.len(),payload_size:image.payload_bytes,import_descriptor:imports,status})
}
pub fn format_report(data:&[u8])->Result<String,String>{
    let r=analyze(data)?;
    Ok(format!("XEX Real Loader 5.1\nHeaders: {}\nImage base: {:?}\nEntry point: {:?}\nPayload: {} bytes\nImport descriptor: {}\nStatus: {:?}\n\nInspecao somente; jogos comerciais nao executam.",r.optional_count,r.image_base,r.entry_point,r.payload_size,r.import_descriptor,r.status))
}
#[cfg(test)] mod tests {use super::*;
    #[test]fn rejects_non_xex(){assert!(analyze(b"INVALID").is_err());}
    #[test]fn detects_imports(){
        let mut b=vec![0u8;128];b[..4].copy_from_slice(b"XEX2");
        b[8..12].copy_from_slice(&96u32.to_be_bytes());b[16..20].copy_from_slice(&64u32.to_be_bytes());b[20..24].copy_from_slice(&4u32.to_be_bytes());
        for (i,(key,val)) in [(0x000103FFu32,80u32),(0x00040006,0x10001000),(0x00030000,0x10001000),(0x000401FF,88)].iter().enumerate(){let p=24+i*8;b[p..p+4].copy_from_slice(&key.to_be_bytes());b[p+4..p+8].copy_from_slice(&val.to_be_bytes());}
        b[80..84].copy_from_slice(&8u32.to_be_bytes());b[86..88].copy_from_slice(&1u16.to_be_bytes());
        let r=analyze(&b).unwrap();assert!(r.import_descriptor);assert!(matches!(r.status,LoadStatus::Unsupported(_)));
    }
}
