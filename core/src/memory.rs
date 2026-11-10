#[derive(Debug)]
pub struct Memory { data: Vec<u8> }
impl Memory {
 pub fn new(bytes: usize)->Self { Self{data:vec![0;bytes]} }
 pub fn len(&self)->usize {self.data.len()}
 pub fn read_u32_be(&self,addr:usize)->Result<u32,&'static str>{
  let b=self.data.get(addr..addr.checked_add(4).ok_or("address overflow")?).ok_or("memory out of range")?;
  Ok(u32::from_be_bytes(b.try_into().unwrap()))
 }
 pub fn write(&mut self,addr:usize,src:&[u8])->Result<(),&'static str>{
  let dst=self.data.get_mut(addr..addr.checked_add(src.len()).ok_or("address overflow")?).ok_or("memory out of range")?;
  dst.copy_from_slice(src);Ok(())
 }
 pub fn read(&self,addr:usize,len:usize)->Result<&[u8],&'static str>{
  self.data.get(addr..addr.checked_add(len).ok_or("address overflow")?).ok_or("memory out of range")
 }
}
#[cfg(test)]mod tests {use super::*;#[test]fn bounds(){let mut m=Memory::new(8);m.write(0,&[0x12,0x34,0x56,0x78]).unwrap();assert_eq!(m.read_u32_be(0).unwrap(),0x12345678);assert!(m.write(7,&[1,2]).is_err());}}
