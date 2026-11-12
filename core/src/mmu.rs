//! Bounded sparse guest page table used for tests and future MMU integration.
//! This models permissions but not Xenon TLB, cache or address translation.
use std::collections::BTreeMap;
pub const BASE:u64=0x8200_0000;
pub const SIZE:u64=512*1024*1024;
const PAGE:usize=4096;
pub const R:u8=1;pub const W:u8=2;pub const X:u8=4;
#[derive(Default,Debug)]
pub struct SparseMmu{pages:BTreeMap<u32,Box<[u8;PAGE]>>,flags:BTreeMap<u32,u8>}
impl SparseMmu{
 pub fn new()->Self{Self::default()}
 fn position(a:u64,n:usize)->Result<(u32,usize),&'static str>{
    if a<BASE||a.checked_add(n as u64).is_none_or(|end|end>BASE+SIZE){return Err("MMU address out of bounds");}
    let rel=a-BASE;Ok(((rel/PAGE as u64) as u32,(rel%PAGE as u64) as usize))
 }
 pub fn map(&mut self,addr:u64,size:usize,perms:u8)->Result<(),&'static str>{
    if size==0||size>64*1024*1024||perms==0||perms&!7!=0{return Err("MMU invalid mapping");}
    let (start,_)=Self::position(addr,size)?;let (last,_)=Self::position(addr+size as u64-1,1)?;
    for page in start..=last{if self.flags.contains_key(&page){return Err("MMU overlapping mapping");}}
    for page in start..=last{self.flags.insert(page,perms);}
    Ok(())
 }
 pub fn unmap(&mut self,addr:u64,size:usize)->Result<(),&'static str>{
    if size==0{return Err("MMU zero size");}
    let (start,_)=Self::position(addr,size)?;let (last,_)=Self::position(addr+size as u64-1,1)?;
    for i in start..=last{self.pages.remove(&i);self.flags.remove(&i);}Ok(())
 }
 pub fn write(&mut self,addr:u64,bytes:&[u8])->Result<(),&'static str>{
    let (first,_)=Self::position(addr,bytes.len())?;
    if bytes.is_empty(){return Ok(());}
    let (last,_)=Self::position(addr+bytes.len() as u64-1,1)?;
    for p in first..=last {if self.flags.get(&p).copied().unwrap_or(0)&W==0{return Err("MMU write denied");}}
    for (i,b) in bytes.iter().enumerate(){
        let (p,off)=Self::position(addr+i as u64,1)?;
        self.pages.entry(p).or_insert_with(||Box::new([0;PAGE]))[off]=*b;
    }
    Ok(())
 }
 pub fn read(&self,addr:u64,size:usize,exec:bool)->Result<Vec<u8>,&'static str>{
    let (first,_)=Self::position(addr,size)?;
    if size>8*1024*1024{return Err("MMU oversized read");}
    if size==0{return Ok(Vec::new());}
    let (last,_)=Self::position(addr+size as u64-1,1)?;
    for p in first..=last {if self.flags.get(&p).copied().unwrap_or(0)&(if exec{X}else{R})==0{return Err("MMU read denied");}}
    let mut bytes=Vec::with_capacity(size);
    for i in 0..size{
        let (p,off)=Self::position(addr+i as u64,1)?;
        bytes.push(self.pages.get(&p).map_or(0,|page|page[off]));
    }
    Ok(bytes)
 }
 pub fn read_be32(&self,addr:u64)->Result<u32,&'static str>{let data=self.read(addr,4,false)?;Ok(u32::from_be_bytes(data.try_into().unwrap()))}
 pub fn committed_bytes(&self)->usize{self.pages.len()*PAGE}
}
#[cfg(test)]mod tests{use super::*;
 #[test]fn sparse_rw_and_permissions(){
 let mut m=SparseMmu::new();m.map(BASE,8192,R|W).unwrap();
 m.write(BASE+4094,&[0x12,0x34,0x56,0x78]).unwrap();
 assert_eq!(m.read_be32(BASE+4094).unwrap(),0x12345678);
 assert_eq!(m.committed_bytes(),8192);assert!(m.read(BASE,4,true).is_err());
 assert!(m.map(BASE,4096,R).is_err());m.unmap(BASE,8192).unwrap();
 assert!(m.read(BASE,4,false).is_err());
 }
}