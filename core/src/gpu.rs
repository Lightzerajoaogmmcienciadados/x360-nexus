/// Placeholder framebuffer: no Xenos command processing or Vulkan backend yet.
#[derive(Debug)]pub struct Gpu {pub width:usize,pub height:usize,rgba:Vec<u8>}
impl Gpu {
 pub fn new(width:usize,height:usize)->Result<Self,&'static str>{let len=width.checked_mul(height).and_then(|v|v.checked_mul(4)).ok_or("framebuffer overflow")?;if len>64*1024*1024{return Err("framebuffer too large")}Ok(Self{width,height,rgba:vec![0;len]})}
 pub fn clear(&mut self,color:[u8;4]){for pixel in self.rgba.chunks_exact_mut(4){pixel.copy_from_slice(&color)}}
 pub fn framebuffer(&self)->&[u8]{&self.rgba}
}
#[cfg(test)]mod tests{use super::*;#[test]fn clear(){let mut g=Gpu::new(2,1).unwrap();g.clear([0,255,0,255]);assert_eq!(g.framebuffer(),&[0,255,0,255,0,255,0,255]);}}
