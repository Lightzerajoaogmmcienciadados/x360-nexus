//! Guest-addressed big-endian memory. Dense storage for small unit tests;
//! sparse 4-KiB pages for the 512-MiB Xenon diagnostic address window.
//! Page permissions are enforced for instruction fetches and guest accesses.
//! This is NOT a Xenon hardware TLB or a full physical-memory controller.
use std::collections::BTreeMap;
const PAGE: usize = 4096;
#[derive(Debug)]
enum Storage { Dense(Vec<u8>), Sparse(BTreeMap<usize,Box<[u8;PAGE]>>) }
#[derive(Debug)]
pub struct Memory { storage: Storage, size: usize, base: u64, perms: Vec<u8> }
impl Memory {
    pub const READ:u8=1; pub const WRITE:u8=2; pub const EXECUTE:u8=4;
    pub fn new(bytes:usize)->Self{Self::with_base(vec![0;bytes],0)}
    pub fn with_base(bytes:Vec<u8>,base:u64)->Self{
        let size=bytes.len();Self{storage:Storage::Dense(bytes),size,base,perms:vec![7;(size+PAGE-1)/PAGE]}
    }
    pub fn sparse(base:u64,size:usize)->Result<Self,&'static str>{
        if size==0||size>512*1024*1024||base.checked_add(size as u64).is_none_or(|v|v>1u64<<32){return Err("invalid sparse RAM window");}
        Ok(Self{storage:Storage::Sparse(BTreeMap::new()),size,base,perms:vec![0;(size+PAGE-1)/PAGE]})
    }
    pub fn len(&self)->usize{self.size}
    pub fn is_sparse(&self)->bool{matches!(&self.storage,Storage::Sparse(_))}
    pub fn unmap_region(&mut self,addr:u64,len:usize)->Result<(),&'static str>{
        let start=self.offset(addr as usize,len)?;
        if len==0{return Ok(());}
        let first=start/PAGE;let last=(start+len-1)/PAGE;
        for p in first..=last{self.perms[p]=0;}
        match &mut self.storage{
            Storage::Dense(bytes)=>bytes[start..start+len].fill(0),
            Storage::Sparse(pages)=>{for page in first..=last{pages.remove(&page);}}
        }
        Ok(())
    }
    pub fn base(&self)->u64{self.base}
    pub fn committed_bytes(&self)->usize{
        match &self.storage { Storage::Dense(v)=>v.len(),Storage::Sparse(p)=>p.len()*PAGE }
    }
    pub fn set_all_permissions(&mut self,mask:u8){self.perms.fill(mask&7);}
    pub fn set_page_permissions(&mut self,addr:u64,len:usize,mask:u8)->Result<(),&'static str>{
        if mask&!7!=0{return Err("invalid page flags");}
        let start=self.offset(addr as usize,len)?;
        if len==0{return Ok(());}
        for p in start/PAGE..=(start+len-1)/PAGE{self.perms[p]=mask;}
        Ok(())
    }
    fn offset(&self,addr:usize,len:usize)->Result<usize,&'static str>{
        let off=(addr as u64).checked_sub(self.base).ok_or("address below guest base")?;
        let off=usize::try_from(off).map_err(|_|"address overflow")?;
        if off.checked_add(len).is_none_or(|end|end>self.size){return Err("memory out of range");}
        Ok(off)
    }
    fn check(&self,off:usize,len:usize,perm:u8)->Result<(),&'static str>{
        if len==0{return Ok(());}
        for p in off/PAGE..=(off+len-1)/PAGE{
            if self.perms[p]&perm!=perm{return Err("guest page permission denied");}
        }
        Ok(())
    }
    fn load_bytes(&self,off:usize,len:usize)->Vec<u8>{
        let mut dst=vec![0;len];
        match &self.storage {
            Storage::Dense(v)=>dst.copy_from_slice(&v[off..off+len]),
            Storage::Sparse(pages)=>{
                let mut pos=0;
                while pos<len{
                    let at=off+pos;let within=at%PAGE;let take=(len-pos).min(PAGE-within);
                    if let Some(page)=pages.get(&(at/PAGE)){
                        dst[pos..pos+take].copy_from_slice(&page[within..within+take]);
                    }
                    pos+=take;
                }
            }
        }
        dst
    }
    fn store_bytes(&mut self,off:usize,src:&[u8]){
        match &mut self.storage {
            Storage::Dense(v)=>v[off..off+src.len()].copy_from_slice(src),
            Storage::Sparse(pages)=>{
                let mut pos=0;
                while pos<src.len(){
                    let at=off+pos;let within=at%PAGE;let take=(src.len()-pos).min(PAGE-within);
                    let chunk=&src[pos..pos+take];let page_number=at/PAGE;
                    if chunk.iter().any(|b|*b!=0) || pages.contains_key(&page_number){
                        pages.entry(page_number).or_insert_with(||Box::new([0;PAGE]))[within..within+take].copy_from_slice(chunk);
                    }
                    pos+=take;
                }
            }
        }
    }
    pub fn read(&self,addr:usize,len:usize)->Result<Vec<u8>,&'static str>{
        let off=self.offset(addr,len)?;self.check(off,len,Self::READ)?;
        Ok(self.load_bytes(off,len))
    }
    pub fn write(&mut self,addr:usize,src:&[u8])->Result<(),&'static str>{
        let off=self.offset(addr,src.len())?;self.check(off,src.len(),Self::WRITE)?;
        self.store_bytes(off,src);Ok(())
    }
    /// Loader writes bypass guest R/W/X after validating the destination region.
    /// The guest CPU cannot call this function.
    pub fn load_image_bytes(&mut self,addr:usize,src:&[u8])->Result<(),&'static str>{
        let off=self.offset(addr,src.len())?;self.store_bytes(off,src);Ok(())
    }
    pub fn fetch_u32_be(&self,addr:usize)->Result<u32,&'static str>{
        let off=self.offset(addr,4)?;self.check(off,4,Self::EXECUTE)?;
        Ok(u32::from_be_bytes(self.load_bytes(off,4).try_into().unwrap()))
    }
    pub fn read_u8(&self,a:usize)->Result<u8,&'static str>{Ok(self.read(a,1)?[0])}
    pub fn read_u16_be(&self,a:usize)->Result<u16,&'static str>{Ok(u16::from_be_bytes(self.read(a,2)?.try_into().unwrap()))}
    pub fn read_u32_be(&self,a:usize)->Result<u32,&'static str>{Ok(u32::from_be_bytes(self.read(a,4)?.try_into().unwrap()))}
    pub fn read_u64_be(&self,a:usize)->Result<u64,&'static str>{Ok(u64::from_be_bytes(self.read(a,8)?.try_into().unwrap()))}
    pub fn write_u8(&mut self,a:usize,v:u8)->Result<(),&'static str>{self.write(a,&[v])}
    pub fn write_u16_be(&mut self,a:usize,v:u16)->Result<(),&'static str>{self.write(a,&v.to_be_bytes())}
    pub fn write_u32_be(&mut self,a:usize,v:u32)->Result<(),&'static str>{self.write(a,&v.to_be_bytes())}
    pub fn write_u64_be(&mut self,a:usize,v:u64)->Result<(),&'static str>{self.write(a,&v.to_be_bytes())}
}
#[cfg(test)] mod tests {use super::*;
 #[test]fn dense_endian(){let mut m=Memory::with_base(vec![0;8],0x82000000);m.write_u32_be(0x82000000,0x12345678).unwrap();assert_eq!(m.read(0x82000000,4).unwrap(),vec![0x12,0x34,0x56,0x78]);assert!(m.read_u32_be(0x82000006).is_err());}
 #[test]fn sparse_permissions_and_pages(){let mut m=Memory::sparse(0x82000000,512*1024*1024).unwrap();m.set_page_permissions(0x82001000,8192,Memory::READ|Memory::WRITE).unwrap();m.write_u32_be(0x82001ffe,0x12345678).unwrap();assert_eq!(m.committed_bytes(),8192);assert_eq!(m.read_u32_be(0x82001ffe).unwrap(),0x12345678);assert!(m.fetch_u32_be(0x82001000).is_err());m.set_page_permissions(0x82001000,8192,Memory::READ|Memory::EXECUTE).unwrap();assert!(m.write_u8(0x82001000,1).is_err());assert_eq!(m.fetch_u32_be(0x82001ffe).unwrap(),0x12345678);}
 #[test]fn sparse_zeros_no_commit(){let mut m=Memory::sparse(0x82000000,512*1024*1024).unwrap();m.load_image_bytes(0x82000000,&[0;65536]).unwrap();assert_eq!(m.committed_bytes(),0);}
}