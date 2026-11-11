//! Read-only, bounded XEX2 optional-header and loader diagnostics.
//! This does NOT decrypt, decompress, relocate or execute Xbox 360 titles.
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionalHeader { pub key:u32, pub value:u32, pub description:&'static str }
#[derive(Debug, Clone)]
pub struct XexReport {
    pub module_flags:u32, pub image_offset:u32, pub header_size:u32,
    pub security_offset:u32, pub optional_headers:Vec<OptionalHeader>,
    pub entry_point:Option<u32>, pub image_base:Option<u32>,
}
fn be32(data:&[u8],offset:usize)->Result<u32,String>{
    let end=offset.checked_add(4).ok_or("offset overflow")?;
    let b=data.get(offset..end).ok_or("XEX2 truncado")?;
    Ok(u32::from_be_bytes(b.try_into().map_err(|_|"XEX2 truncado")?))
}
fn description(key:u32)->&'static str { match key {
    0x00010100=>"System flags",0x00010201=>"Resource info",
    0x000103FF=>"Base file format",0x00018002=>"Delta patch descriptor",
    0x000183FF=>"Delta patch info",0x000200FF=>"Execution info",
    0x00020104=>"Title workspace size",0x00020200=>"Game ratings",
    0x00020301=>"LAN key",0x00020401=>"Xbox 360 logo",
    0x00030000=>"Image base address",0x00040006=>"Entry point",
    0x000401FF=>"Image imports",0x00040201=>"Checksum timestamp",
    0x00040310=>"Original PE name",0x00040404=>"Static libraries",
    0x000405FF=>"TLS info",0x000406FF=>"Default stack size",
    0x000407FF=>"Default filesystem cache",0x00040801=>"Default heap size",
    0x000409FF=>"Page heap size",0x00040A01=>"System info",
    _=>"Unknown/other optional header"
}}
/// Parses only fields inside the provided XEX header bytes. No arbitrary code execution.
pub fn parse(data:&[u8])->Result<XexReport,String>{
    if data.get(..4)!=Some(b"XEX2".as_slice()){return Err("Assinatura XEX2 ausente".into())}
    if data.len()<24{return Err("Cabecalho menor que 24 bytes".into())}
    let flags=be32(data,4)?;
    let image_offset=be32(data,8)?;
    let security_offset=be32(data,16)?;
    let count=be32(data,20)? as usize;
    if count>4096{return Err("Numero excessivo de optional headers".into())}
    let table_end=24usize.checked_add(count.checked_mul(8).ok_or("Tabela overflow")?).ok_or("Tabela overflow")?;
    if table_end>data.len(){return Err("Tabela de optional headers truncada".into())}
    let mut optional_headers=Vec::with_capacity(count);
    let mut seen=HashSet::new();
    let mut entry_point=None;let mut image_base=None;
    for i in 0..count {
        let at=24+i*8;let key=be32(data,at)?;let value=be32(data,at+4)?;
        if !seen.insert(key){return Err(format!("Optional header duplicado: 0x{key:08X}"))}
        if key==0x00040006 {entry_point=Some(value)}
        if key==0x00030000 {image_base=Some(value)}
        optional_headers.push(OptionalHeader{key,value,description:description(key)});
    }
    Ok(XexReport{module_flags:flags,image_offset,header_size:image_offset,security_offset,optional_headers,entry_point,image_base})
}
pub fn format_report(data:&[u8],file_size:u64)->Result<String,String>{
    let report=parse(data)?;
    if report.image_offset as u64>file_size{return Err("Image offset fora do XEX".into())}
    if report.security_offset as u64>file_size{return Err("Security offset fora do XEX".into())}
    let mut s=format!("PROJECT XENON 4.4 — XEX LOADER DIAGNOSTICO\n\nDefault.xex: {file_size} bytes\nModule flags: 0x{:08X}\nImage offset: 0x{:08X}\nSecurity offset: 0x{:08X}\nOptional headers: {}\n",report.module_flags,report.image_offset,report.security_offset,report.optional_headers.len());
    if let Some(v)=report.entry_point{s.push_str(&format!("Entry point declarado: 0x{v:08X}\n"))}
    if let Some(v)=report.image_base{s.push_str(&format!("Image base declarada: 0x{v:08X}\n"))}
    s.push_str("\nCabecalhos opcionais:\n");
    for h in &report.optional_headers {s.push_str(&format!("0x{:08X} = 0x{:08X} | {}\n",h.key,h.value,h.description))}
    s.push_str("\nDiagnostico somente. Descriptografia, descompressao, imports, kernel e execucao de jogos NAO implementados.");
    Ok(s)
}
#[cfg(test)]mod tests{use super::*;
 fn sample()->Vec<u8>{let mut b=vec![0u8;80];b[..4].copy_from_slice(b"XEX2");b[8..12].copy_from_slice(&80u32.to_be_bytes());b[16..20].copy_from_slice(&64u32.to_be_bytes());b[20..24].copy_from_slice(&2u32.to_be_bytes());b[24..28].copy_from_slice(&0x00040006u32.to_be_bytes());b[28..32].copy_from_slice(&0x82000000u32.to_be_bytes());b[32..36].copy_from_slice(&0x00030000u32.to_be_bytes());b[36..40].copy_from_slice(&0x82000000u32.to_be_bytes());b}
 #[test]fn fields(){let p=parse(&sample()).unwrap();assert_eq!(p.entry_point,Some(0x82000000));assert_eq!(p.optional_headers.len(),2)}
 #[test]fn truncated(){assert!(parse(&sample()[..30]).is_err())}
 #[test]fn invalid(){assert!(parse(b"ABCD").is_err())}
 #[test]fn duplicate(){let mut b=sample();b[32..36].copy_from_slice(&0x00040006u32.to_be_bytes());assert!(parse(&b).is_err())}
 #[test]fn bounds(){assert!(format_report(&sample(),70).is_err())}
}
