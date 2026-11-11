//! XEX image layout inspection (read-only). No commercial game execution.
use crate::xex_loader::parse;

#[derive(Debug, PartialEq, Eq)]
pub enum StorageKind { Uncompressed, Compressed, DeltaPatch, Unknown(u16) }
#[derive(Debug)]
pub struct ImageLayout {
    pub image_offset: u32,
    pub security_offset: u32,
    pub entry_point: Option<u32>,
    pub image_base: Option<u32>,
    pub storage: Option<StorageKind>,
    pub encryption_type: Option<u16>,
    pub payload_bytes: usize,
    pub executable_ready: bool,
}
fn be16(buf:&[u8],pos:usize)->Result<u16,String>{let s=buf.get(pos..pos.checked_add(2).ok_or("overflow")?).ok_or("Campo XEX truncado")?;Ok(u16::from_be_bytes([s[0],s[1]]))}
/// Analyzes the base-file-format descriptor at optional header 0x000103FF.
/// Values are reported only if the descriptor exists in the supplied bytes.
pub fn inspect(data:&[u8])->Result<ImageLayout,String>{
    if data.len()>32*1024*1024{return Err("Arquivo acima do limite de analise (32 MiB)".into())}
    let h=parse(data)?;
    let offset=h.image_offset as usize;
    if offset>data.len(){return Err("Image offset fora do arquivo".into())}
    if h.security_offset as usize>data.len(){return Err("Security offset fora do arquivo".into())}
    let mut storage=None;let mut encryption_type=None;
    if let Some(opt)=h.optional_headers.iter().find(|x|x.key==0x000103FF){
        let p=opt.value as usize;
        if p<24 || p.checked_add(8).filter(|end|*end<=data.len()).is_none(){return Err("Descritor base-file-format invalido".into())}
        let length=u32::from_be_bytes(data[p..p+4].try_into().map_err(|_|"Descritor invalido")?) as usize;
        if length<8 || p.checked_add(length).filter(|end|*end<=data.len()).is_none(){return Err("Descritor base-file-format truncado".into())}
        encryption_type=Some(be16(data,p+4)?);
        let kind=be16(data,p+6)?;
        storage=Some(match kind{1=>StorageKind::Uncompressed,2=>StorageKind::Compressed,3=>StorageKind::DeltaPatch,other=>StorageKind::Unknown(other)});
    }
    // No executable image mapping or key handling exists here.
    Ok(ImageLayout{image_offset:h.image_offset,security_offset:h.security_offset,entry_point:h.entry_point,image_base:h.image_base,storage,encryption_type,payload_bytes:data.len()-offset,executable_ready:false})
}
pub fn diagnostic(data:&[u8])->Result<String,String>{let r=inspect(data)?;Ok(format!(
    "XEX Image Engine 4.9: layout analisado\\nImage offset: 0x{:08X}\\nSecurity offset: 0x{:08X}\\nEntry point declarado: {:?}\\nImage base declarada: {:?}\\nArmazenamento: {:?}\\nCriptografia (codigo): {:?}\\nBytes de payload: {}\\nImagem executavel: NAO (mapeamento, imports e GPU ausentes)",
    r.image_offset,r.security_offset,r.entry_point,r.image_base,r.storage,r.encryption_type,r.payload_bytes).replace("\\n","\n"))}
pub fn demo()->Result<String,String>{
    let mut b=vec![0u8;128];b[0..4].copy_from_slice(b"XEX2");b[8..12].copy_from_slice(&96u32.to_be_bytes());b[16..20].copy_from_slice(&64u32.to_be_bytes());b[20..24].copy_from_slice(&3u32.to_be_bytes());
    for (i,(key,val)) in [(0x000103FFu32,72u32),(0x00040006,0x10001000),(0x00030000,0x10000000)].iter().enumerate(){let p=24+i*8;b[p..p+4].copy_from_slice(&key.to_be_bytes());b[p+4..p+8].copy_from_slice(&val.to_be_bytes());}
    b[72..76].copy_from_slice(&8u32.to_be_bytes());b[76..78].copy_from_slice(&0u16.to_be_bytes());b[78..80].copy_from_slice(&1u16.to_be_bytes());diagnostic(&b)
}
#[cfg(test)]mod tests{use super::*;
#[test]fn synthetic_uncompressed(){let s=demo().unwrap();assert!(s.contains("Uncompressed"));assert!(s.contains("NAO"));}
#[test]fn rejects_invalid(){assert!(inspect(b"BAD!").is_err());}
#[test]fn rejects_truncated_descriptor(){let mut b=vec![0u8;64];b[0..4].copy_from_slice(b"XEX2");b[8..12].copy_from_slice(&64u32.to_be_bytes());b[20..24].copy_from_slice(&1u32.to_be_bytes());b[24..28].copy_from_slice(&0x000103FFu32.to_be_bytes());b[28..32].copy_from_slice(&63u32.to_be_bytes());assert!(inspect(&b).is_err());}
}
