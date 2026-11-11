//! Sparse, bounded virtual address space for experimental PPC homebrew.
//! No mapping of encrypted XEX executable data or Xbox 360 MMU emulation.
use std::collections::BTreeMap;
pub const PAGE_SIZE:usize=4096;
pub const READ:u8=1;
pub const WRITE:u8=2;
pub const EXEC:u8=4;
const MAX_PAGES:usize=4096; // 16 MiB safety limit
#[derive(Clone)]struct Page{data:Box<[u8;PAGE_SIZE]>,perm:u8}
pub struct VirtualMemory{pages:BTreeMap<u32,Page>}
impl VirtualMemory{
 pub fn new()->Self{Self{pages:BTreeMap::new()}}
 pub fn mapped_pages(&self)->usize{self.pages.len()}
 pub fn map(&mut self,start:u32,size:usize,perm:u8)->Result<(),String>{
  if size==0||start as usize%PAGE_SIZE!=0||size%PAGE_SIZE!=0||perm==0||perm & !7!=0{return Err("invalid mapping parameters".into())}
  let count=size/PAGE_SIZE;
  if count>MAX_PAGES||self.pages.len().saturating_add(count)>MAX_PAGES{return Err("memory budget exceeded".into())}
  let end=(start as u64).checked_add(size as u64).ok_or("address overflow")?;
  if end>u32::MAX as u64+1{return Err("address overflow".into())}
  for i in 0..count {let a=(start as u64+i as u64*PAGE_SIZE as u64) as u32;if self.pages.contains_key(&a){return Err("overlapping mapping".into())}}
  for i in 0..count {let a=(start as u64+i as u64*PAGE_SIZE as u64) as u32;self.pages.insert(a,Page{data:Box::new([0;PAGE_SIZE]),perm});}Ok(())
 }
 fn locate(addr:u32)->(u32,usize){(addr & !((PAGE_SIZE as u32)-1),(addr as usize)%PAGE_SIZE)}
 pub fn read(&self,addr:u32,len:usize,access:u8)->Result<Vec<u8>,String>{
  if access!=READ&&access!=EXEC{return Err("invalid access".into())}
  if len>MAX_PAGES*PAGE_SIZE{return Err("read too large".into())}
  let end=(addr as u64).checked_add(len as u64).ok_or("address overflow")?;
  if end>u32::MAX as u64+1{return Err("address overflow".into())}
  let mut out=Vec::with_capacity(len);
  for i in 0..len{let (base,offset)=Self::locate((addr as u64+i as u64) as u32);let p=self.pages.get(&base).ok_or("unmapped address")?;if p.perm&access==0{return Err("access denied".into())}out.push(p.data[offset]);}Ok(out)
 }
 pub fn write(&mut self,addr:u32,bytes:&[u8])->Result<(),String>{
  let end=(addr as u64).checked_add(bytes.len() as u64).ok_or("address overflow")?;
  if end>u32::MAX as u64+1{return Err("address overflow".into())}
  // Validate entire range before mutating anything.
  for i in 0..bytes.len(){let (base,_)=Self::locate((addr as u64+i as u64) as u32);let p=self.pages.get(&base).ok_or("unmapped address")?;if p.perm&WRITE==0{return Err("access denied".into())}}
  for (i,b) in bytes.iter().enumerate(){let (base,offset)=Self::locate((addr as u64+i as u64) as u32);self.pages.get_mut(&base).unwrap().data[offset]=*b;}Ok(())
 }
 pub fn read_u32_be(&self,addr:u32,access:u8)->Result<u32,String>{let b=self.read(addr,4,access)?;Ok(u32::from_be_bytes(b.try_into().unwrap()))}
}
impl Default for VirtualMemory{fn default()->Self{Self::new()}}
pub fn demo()->Result<String,String>{let mut m=VirtualMemory::new();m.map(0x1000,8192,READ|WRITE|EXEC)?;m.write(0x1ffe,&[0x38,0x60,0,42])?;let op=m.read_u32_be(0x1ffe,EXEC)?;if op!=0x3860002a{return Err("self-test mismatch".into())}Ok(format!("Memory Engine OK: {} pages, PPC opcode 0x{:08X}, cross-page access",m.mapped_pages(),op))}
#[cfg(test)]mod tests{use super::*;
 #[test]fn crossing_pages(){assert!(demo().is_ok())}
 #[test]fn permissions(){let mut m=VirtualMemory::new();m.map(0x1000,4096,READ).unwrap();assert!(m.write(0x1000,&[1]).is_err());assert!(m.read(0x1000,1,EXEC).is_err())}
 #[test]fn overlap(){let mut m=VirtualMemory::new();m.map(0x1000,4096,READ).unwrap();assert!(m.map(0x1000,4096,READ).is_err())}
 #[test]fn atomic_write(){let mut m=VirtualMemory::new();m.map(0x1000,4096,READ|WRITE).unwrap();assert!(m.write(0x1fff,&[1,2]).is_err());assert_eq!(m.read(0x1fff,1,READ).unwrap(),vec![0])}
 #[test]fn bounds(){let mut m=VirtualMemory::new();assert!(m.map(0xfffff000,8192,READ).is_err());assert!(m.map(0,1,READ).is_err())}
}
