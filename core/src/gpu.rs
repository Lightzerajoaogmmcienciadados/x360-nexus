//! Software RGBA framebuffer for diagnostics, NOT Xbox 360 Xenos graphics.
#[derive(Debug)]
pub struct Gpu { pub width: usize, pub height: usize, rgba: Vec<u8> }
impl Gpu {
    pub fn new(width:usize,height:usize)->Result<Self,&'static str>{
        let len=width.checked_mul(height).and_then(|n|n.checked_mul(4)).ok_or("framebuffer overflow")?;
        if width==0 || height==0 || len>64*1024*1024 { return Err("invalid framebuffer size"); }
        Ok(Self{width,height,rgba:vec![0;len]})
    }
    pub fn clear(&mut self, color:[u8;4]) {
        for p in self.rgba.chunks_exact_mut(4){p.copy_from_slice(&color)}
    }
    pub fn pixel(&mut self,x:usize,y:usize,color:[u8;4]) {
        if x>=self.width||y>=self.height{return}
        let i=(y*self.width+x)*4;self.rgba[i..i+4].copy_from_slice(&color);
    }
    pub fn rect(&mut self,x:usize,y:usize,w:usize,h:usize,color:[u8;4]){
        for yy in y.min(self.height)..y.saturating_add(h).min(self.height){
            for xx in x.min(self.width)..x.saturating_add(w).min(self.width){ self.pixel(xx,yy,color); }
        }
    }
    pub fn framebuffer(&self)->&[u8]{&self.rgba}
}
#[cfg(test)]mod tests{use super::*;#[test]fn framebuffer(){let mut gpu=Gpu::new(8,8).unwrap();gpu.clear([1,2,3,255]);gpu.rect(1,1,2,2,[20,99,30,255]);assert_eq!(&gpu.framebuffer()[36..40],&[20,99,30,255]);}}
