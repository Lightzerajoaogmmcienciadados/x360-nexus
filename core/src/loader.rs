//! Read-only XDVDFS discovery and conservative XEX2 metadata inspection.
use std::io::{Read,Seek,SeekFrom};
const MAGIC:&[u8;20]=b"MICROSOFT*XBOX*MEDIA";
const SECTOR:u64=2048;
#[derive(Debug,PartialEq,Eq)]pub enum FileKind{Xex2,Unknown}
pub fn identify(header:&[u8])->FileKind{if header.starts_with(b"XEX2"){FileKind::Xex2}else{FileKind::Unknown}}
#[derive(Debug,Clone)]pub struct IsoInfo{pub layout:&'static str,pub partition:u64,pub root_sector:u32,pub root_size:u32,pub image_size:u64}
fn u32le(b:&[u8])->u32{u32::from_le_bytes(b.try_into().unwrap())}
pub fn inspect_iso<R:Read+Seek>(r:&mut R)->Result<IsoInfo,String>{
 let size=r.seek(SeekFrom::End(0)).map_err(|e|e.to_string())?;
 // Layout candidates; not every Xbox disc dump uses these offsets.
 for (name,base) in [("XISO",0u64),("XGD2 candidate",0x0FD90000),("XGD3 candidate",0x02080000),("XGD1 candidate",0x18300000)]{
  let Some(off)=base.checked_add(32*SECTOR) else{continue};
  if off.checked_add(SECTOR).is_none_or(|end|end>size){continue}
  r.seek(SeekFrom::Start(off)).map_err(|e|e.to_string())?;
  let mut d=[0u8;2048];r.read_exact(&mut d).map_err(|e|e.to_string())?;
  if &d[..20]!=MAGIC || &d[2028..]!=MAGIC{continue}
  let root_sector=u32le(&d[20..24]);let root_size=u32le(&d[24..28]);
  let Some(start)=base.checked_add((root_sector as u64)*SECTOR) else{continue};
  if root_size==0 || start.checked_add(root_size as u64).is_none_or(|end|end>size){continue}
  return Ok(IsoInfo{layout:name,partition:base,root_sector,root_size,image_size:size})
 }
 Err("Descritor XDVDFS nao encontrado; ISO nao reconhecida".into())
}
#[derive(Debug,Clone)]pub struct DiscEntry{pub name:String,pub is_directory:bool,pub start_sector:u32,pub size:u32}
/// Reads root directory tree records; conservative traversal with bounds and cycle guards.
pub fn root_entries<R:Read+Seek>(r:&mut R,iso:&IsoInfo)->Result<Vec<DiscEntry>,String>{
 let base=iso.partition.checked_add(iso.root_sector as u64*SECTOR).ok_or("overflow")?;
 if iso.root_size>16*1024*1024{return Err("Diretorio grande demais".into())}
 r.seek(SeekFrom::Start(base)).map_err(|e|e.to_string())?;
 let mut buf=vec![0u8;iso.root_size as usize];r.read_exact(&mut buf).map_err(|e|e.to_string())?;
 let mut out=Vec::new();let mut stack=vec![0usize];let mut visited=std::collections::HashSet::new();
 while let Some(pos)=stack.pop(){
  if !visited.insert(pos){continue}if visited.len()>10000{return Err("Limite de entradas excedido".into())}
  if pos+14>buf.len(){return Err("Registro fora do diretorio".into())}
  let left=u16::from_le_bytes([buf[pos],buf[pos+1]]) as usize*4;
  let right=u16::from_le_bytes([buf[pos+2],buf[pos+3]]) as usize*4;
  let sector=u32le(&buf[pos+4..pos+8]);let size=u32le(&buf[pos+8..pos+12]);
  let flags=buf[pos+12];let len=buf[pos+13] as usize;
  if len==0 || pos+14+len>buf.len(){return Err("Nome de entrada invalido".into())}
  let name=String::from_utf8_lossy(&buf[pos+14..pos+14+len]).into_owned();
  out.push(DiscEntry{name,is_directory:flags&0x10!=0,start_sector:sector,size});
  if right!=0{stack.push(right)}if left!=0{stack.push(left)}
 }
 Ok(out)
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XexOptionalHeader {
    pub key:u32,
    pub value:u32,
}
/// Only decodes the raw optional-header table, not encrypted image data.
/// Extracts Xbox 360 Title ID from optional XEX execution-info metadata (0x00040006).
/// This is an 8-character hexadecimal identifier, not evidence of runtime compatibility.
pub fn inspect_xex_title_id(data:&[u8])->Option<u32> {
    let options=inspect_xex_optionals(data).ok()?;
    let execution=options.iter().find(|option|option.key==0x0004_0006)?;
    let offset=usize::try_from(execution.value).ok()?;
    let start=offset.checked_add(12)?;
    let end=start.checked_add(4)?;
    let bytes=data.get(start..end)?;
    let id=u32::from_be_bytes(bytes.try_into().ok()?);
    if id==0 {None} else {Some(id)}
}
pub fn inspect_xex_optionals(data:&[u8])->Result<Vec<XexOptionalHeader>,String>{
    let header=inspect_xex(data)?;
    if header.optional_headers>1024 {
        return Err("Too many XEX optional headers".into());
    }
    let count=header.optional_headers as usize;
    let total=count.checked_mul(8).and_then(|n|n.checked_add(24))
        .ok_or("Optional header table overflow")?;
    if total>data.len(){
        return Err("Tabela XEX opcional truncada".into());
    }
    let mut items=Vec::with_capacity(count);
    for chunk in data[24..total].chunks_exact(8) {
        items.push(XexOptionalHeader {
            key:u32::from_be_bytes(chunk[0..4].try_into().unwrap()),
            value:u32::from_be_bytes(chunk[4..8].try_into().unwrap()),
        });
    }
    Ok(items)
}
#[derive(Debug,Clone)]pub struct XexInfo{pub module_flags:u32,pub header_size:u32,pub security_offset:u32,pub optional_headers:u32}
pub fn inspect_xex(data:&[u8])->Result<XexInfo,String>{
 if data.len()<24||identify(data)!=FileKind::Xex2{return Err("Cabecalho XEX2 invalido".into())}
 let be=|a:usize|u32::from_be_bytes(data[a..a+4].try_into().unwrap());
 let info=XexInfo{module_flags:be(4),header_size:be(8),security_offset:be(16),optional_headers:be(20)};
 if info.optional_headers>4096{return Err("Contagem de cabecalhos suspeita".into())}
 Ok(info)
}
#[cfg(test)]mod tests{use super::*;use std::io::Cursor;
 #[test]fn iso(){let mut v=vec![0;131072];v[65536..65556].copy_from_slice(MAGIC);v[67564..67584].copy_from_slice(MAGIC);v[65556..65560].copy_from_slice(&33u32.to_le_bytes());v[65560..65564].copy_from_slice(&32u32.to_le_bytes());assert_eq!(inspect_iso(&mut Cursor::new(v)).unwrap().root_sector,33)}
 #[test]fn xex(){let mut v=vec![0;24];v[..4].copy_from_slice(b"XEX2");assert!(inspect_xex(&v).is_ok())}
}


/// Result of locating and inspecting the XEX2 header inside an XDVDFS disc image.
/// This is metadata inspection only; it does NOT decrypt, map or execute a game.
#[derive(Debug, Clone)]
pub struct DiscXexInfo {
    pub name: String,
    pub file_size: u32,
    pub file_offset: u64,
    pub xex: XexInfo,
    pub optionals: Vec<XexOptionalHeader>,
    pub title_id: Option<u32>,
}

/// Scans the already validated root directory for default.xex, then reads a
/// bounded header directly from the user's ISO without copying the whole disc.
pub fn inspect_disc_default_xex<R: Read + Seek>(
    reader: &mut R,
    iso: &IsoInfo,
    entries: &[DiscEntry],
) -> Result<Option<DiscXexInfo>, String> {
    let candidate = entries.iter().find(|entry| {
        !entry.is_directory && entry.name.eq_ignore_ascii_case("default.xex")
    });
    let entry = match candidate {
        Some(found) => found,
        None => return Ok(None),
    };
    if entry.size < 24 {
        return Err("default.xex menor que um cabecalho XEX2".into());
    }
    let offset = iso.partition
        .checked_add((entry.start_sector as u64).checked_mul(SECTOR)
            .ok_or("Setor XEX fora do limite")?)
        .ok_or("Posicao de XEX fora do limite")?;
    let end = offset.checked_add(entry.size as u64)
        .ok_or("Tamanho XEX invalido")?;
    if end > iso.image_size {
        return Err("O arquivo XEX ultrapassa os limites da ISO".into());
    }
    let read_len = (entry.size as usize).min(16 * 1024);
    let mut header = vec![0u8; read_len];
    reader.seek(SeekFrom::Start(offset)).map_err(|e|e.to_string())?;
    reader.read_exact(&mut header).map_err(|e|e.to_string())?;
    let xex = inspect_xex(&header)?;
    let optionals = inspect_xex_optionals(&header)?;
    let title_id=inspect_xex_title_id(&header);
    Ok(Some(DiscXexInfo {
        name: entry.name.clone(),
        file_size: entry.size,
        file_offset: offset,
        xex,
        optionals,
        title_id,
    }))
}

#[cfg(test)]
mod disc_boot_tests {
    use super::*;
    use std::io::Cursor;
    fn disc() -> (Vec<u8>, IsoInfo, Vec<DiscEntry>) {
        let mut bytes = vec![0u8; 131072];
        bytes[65536..65556].copy_from_slice(MAGIC);
        bytes[67564..67584].copy_from_slice(MAGIC);
        bytes[65556..65560].copy_from_slice(&33u32.to_le_bytes());
        bytes[65560..65564].copy_from_slice(&32u32.to_le_bytes());
        // The root record points to the next sector, not to the directory itself.
        let rec = 33 * 2048;
        bytes[rec + 4..rec + 8].copy_from_slice(&34u32.to_le_bytes());
        bytes[rec + 8..rec + 12].copy_from_slice(&512u32.to_le_bytes());
        bytes[rec + 13] = 11;
        bytes[rec + 14..rec + 25].copy_from_slice(b"default.xex");
        let xex_offset = 34 * 2048;
        bytes[xex_offset..xex_offset + 4].copy_from_slice(b"XEX2");
        bytes[xex_offset + 8..xex_offset + 12].copy_from_slice(&0x180u32.to_be_bytes());
        let mut reader = Cursor::new(bytes.clone());
        let info = inspect_iso(&mut reader).unwrap();
        let entries = root_entries(&mut reader, &info).unwrap();
        (bytes, info, entries)
    }
    #[test]
    fn discovers_default_xex_inside_iso_without_reading_full_disc() {
        let (bytes, info, entries) = disc();
        let mut reader = Cursor::new(bytes);
        let xex = inspect_disc_default_xex(&mut reader, &info, &entries).unwrap().unwrap();
        assert_eq!(xex.name, "default.xex");
        assert_eq!(xex.file_size, 512);
        assert_eq!(xex.file_offset, 34 * 2048);
        assert_eq!(xex.xex.header_size, 0x180);
    }
    #[test]
    fn rejects_xex_file_extent_outside_disc() {
        let (bytes, info, mut entries) = disc();
        entries[0].start_sector = 150;
        let mut reader = Cursor::new(bytes);
        assert!(inspect_disc_default_xex(&mut reader, &info, &entries).is_err());
    }
    #[test]
    fn no_default_xex_is_not_falsely_reported() {
        let (bytes, info, mut entries) = disc();
        entries[0].name = "other.xex".into();
        assert!(inspect_disc_default_xex(
            &mut Cursor::new(bytes), &info, &entries
        ).unwrap().is_none());
    }
}

#[cfg(test)] mod extra_xex_tests {
    use super::*;
    #[test] fn extracts_two_optional_headers(){
        let mut xex=vec![0u8;40];
        xex[0..4].copy_from_slice(b"XEX2");
        xex[20..24].copy_from_slice(&2u32.to_be_bytes());
        xex[24..28].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        xex[28..32].copy_from_slice(&0x8200_0000u32.to_be_bytes());
        xex[32..36].copy_from_slice(&0x0002_0001u32.to_be_bytes());
        xex[36..40].copy_from_slice(&0x0000_1000u32.to_be_bytes());
        let items=inspect_xex_optionals(&xex).unwrap();
        assert_eq!(items.len(),2);
        assert_eq!(items[0].value,0x8200_0000);
        assert_eq!(items[1].key,0x0002_0001);
    }
    #[test] fn never_reads_truncated_optional_table(){
        let mut xex=vec![0u8;24];xex[0..4].copy_from_slice(b"XEX2");
        xex[20..24].copy_from_slice(&1u32.to_be_bytes());
        assert!(inspect_xex_optionals(&xex).is_err());
    }
}

#[cfg(test)]mod title_id_tests{
    use super::*;
    #[test]fn can_read_execution_title_id(){
        let mut b=vec![0u8;128];b[..4].copy_from_slice(b"XEX2");
        b[20..24].copy_from_slice(&1u32.to_be_bytes());
        b[24..28].copy_from_slice(&0x00040006u32.to_be_bytes());
        b[28..32].copy_from_slice(&64u32.to_be_bytes());
        b[76..80].copy_from_slice(&0x4d530805u32.to_be_bytes());
        assert_eq!(inspect_xex_title_id(&b),Some(0x4d530805));
    }
    #[test]fn truncated_title_id_does_not_panic(){
        let mut b=vec![0u8;32];b[..4].copy_from_slice(b"XEX2");
        b[20..24].copy_from_slice(&1u32.to_be_bytes());
        b[24..28].copy_from_slice(&0x00040006u32.to_be_bytes());
        b[28..32].copy_from_slice(&100u32.to_be_bytes());
        assert_eq!(inspect_xex_title_id(&b),None);
    }
}

/// Read-only identification of STFS package signature. It is metadata detection,
/// not unlocking, decryption, mounting or boot of XBLA/GOD games.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum PackageKind{Con,Live,Pirs,Xex2,Unknown}
pub fn identify_package(bytes:&[u8])->PackageKind{
    if bytes.starts_with(b"CON "){PackageKind::Con}
    else if bytes.starts_with(b"LIVE"){PackageKind::Live}
    else if bytes.starts_with(b"PIRS"){PackageKind::Pirs}
    else if bytes.starts_with(b"XEX2"){PackageKind::Xex2}
    else {PackageKind::Unknown}
}
pub fn package_report(bytes:&[u8])->String{
    match identify_package(bytes){
        PackageKind::Con=>"STFS CON identificado: pacote de conteudo, assinatura nao verificada; leitura de metadados apenas".into(),
        PackageKind::Live=>"STFS LIVE identificado: conteudo Xbox 360, sem descriptografia ou execucao".into(),
        PackageKind::Pirs=>"STFS PIRS identificado: formato de pacote, sem execucao".into(),
        PackageKind::Xex2=>"XEX2 reconhecido: cabeçalho de executavel Xbox 360, sem inicializacao".into(),
        PackageKind::Unknown=>"Formato nao identificado. Arquivo mantido no catalogo, sem execucao".into(),
    }
}
#[cfg(test)]mod package_tests{
    use super::*;
    #[test]fn recognizes_stfs_headers(){
        assert_eq!(identify_package(b"CON \0\0\0\0"),PackageKind::Con);
        assert_eq!(identify_package(b"LIVE\0\0\0"),PackageKind::Live);
        assert_eq!(identify_package(b"PIRS\0\0\0"),PackageKind::Pirs);
        assert_eq!(identify_package(b"XEX2"),PackageKind::Xex2);
        assert_eq!(identify_package(b"bad"),PackageKind::Unknown);
    }
}
