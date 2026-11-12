//! Big-endian guest-addressed memory with explicit bounds checks.
//! The compact 32 MiB snapshot limit is a v4.5 diagnostic guard,
//! not the Xenon's complete 512 MiB MMU.
#[derive(Debug)]
pub struct Memory { data: Vec<u8>, base: u64, perms: Vec<u8> }
impl Memory {
    pub fn new(bytes: usize) -> Self { Self { data: vec![0; bytes], base: 0, perms: vec![7; (bytes+4095)/4096] } }
    pub fn with_base(bytes: Vec<u8>, base: u64) -> Self { Self { perms: vec![7; (bytes.len()+4095)/4096], data: bytes, base } }
    pub const READ:u8=1;
    pub const WRITE:u8=2;
    pub const EXECUTE:u8=4;
    pub fn len(&self)->usize {self.data.len()}
    pub fn set_page_permissions(&mut self,addr:u64,len:usize,mask:u8)->Result<(),&'static str>{
        if mask&!7!=0{return Err("invalid page flags");}
        let start=self.offset(addr as usize,len)?;
        if len==0{return Ok(());}
        for page in start/4096..=(start+len-1)/4096 {self.perms[page]=mask;}
        Ok(())
    }
    pub fn set_all_permissions(&mut self,mask:u8){self.perms.fill(mask&7);}
    fn check(&self,start:usize,len:usize,mask:u8)->Result<(),&'static str>{
        if len==0{return Ok(());}
        for page in start/4096..=(start+len-1)/4096 {
            if self.perms[page]&mask!=mask{return Err("guest page permission denied");}
        }
        Ok(())
    }
    pub fn fetch_u32_be(&self,addr:usize)->Result<u32,&'static str>{
        let p=self.offset(addr,4)?;self.check(p,4,Self::EXECUTE)?;
        self.read_u32_be(addr)
    }
    pub fn base(&self)->u64 {self.base}
    fn offset(&self, addr: usize, len:usize)->Result<usize,&'static str>{
        let at=(addr as u64).checked_sub(self.base).ok_or("address below guest base")?;
        let off=usize::try_from(at).map_err(|_|"guest address overflow")?;
        let end=off.checked_add(len).ok_or("address overflow")?;
        if end>self.data.len(){return Err("memory out of range");}
        Ok(off)
    }
    pub fn read(&self,addr:usize,len:usize)->Result<&[u8],&'static str>{
        let p=self.offset(addr,len)?;self.check(p,len,Self::READ)?;Ok(&self.data[p..p+len])
    }
    pub fn write(&mut self,addr:usize,src:&[u8])->Result<(),&'static str>{
        let p=self.offset(addr,src.len())?;self.check(p,src.len(),Self::WRITE)?;self.data[p..p+src.len()].copy_from_slice(src);Ok(())
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
#[test]fn page_permissions(){
    let mut m=Memory::with_base(vec![0;8192],0x82000000);
    m.write(0x82000000,&[0x60,0,0,0]).unwrap();
    m.set_all_permissions(Memory::READ|Memory::WRITE);
    assert!(m.fetch_u32_be(0x82000000).is_err());
    m.set_page_permissions(0x82000000,4096,Memory::READ|Memory::EXECUTE).unwrap();
    assert_eq!(m.fetch_u32_be(0x82000000).unwrap(),0x60000000);
    assert!(m.write(0x82000000,&[1]).is_err());
}
#[test]fn endian_and_bounds(){let mut m=Memory::with_base(vec![0;8],0x82000000);m.write_u32_be(0x82000000,0x12345678).unwrap();assert_eq!(m.read(0x82000000,4).unwrap(), &[0x12,0x34,0x56,0x78]);assert_eq!(m.read_u16_be(0x82000002).unwrap(),0x5678);assert!(m.read_u32_be(0x82000006).is_err());}}