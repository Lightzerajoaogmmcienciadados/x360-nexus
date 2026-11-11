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
