//! Xenos-inspired experimental *software* command processor.
//!
//! This intentionally DOES NOT decode Xenos PM4 packets, Xenon memory,
//! Xbox shader microcode, edram, tiling, or proprietary GPU registers.
//! It provides a bounded teaching/diagnostic pipeline with framebuffer,
//! scissors, software triangle rasterization, alpha blending and sampling
//! an original procedural 2x2 texture on Android.
const LIMIT:usize = 512;
const MAX_COMMANDS:usize = 1024;

#[derive(Debug, Clone, Copy)]
pub enum Command {
    Clear(u32),
    Scissor{x:i32,y:i32,w:i32,h:i32},
    Rect{x:i32,y:i32,w:i32,h:i32,color:u32},
    Blend{x:i32,y:i32,w:i32,h:i32,color:u32},
    Triangle{a:(i32,i32),b:(i32,i32),c:(i32,i32),color:u32},
    Texture{x:i32,y:i32,w:i32,h:i32},
}

#[derive(Debug, Default,Clone)]
pub struct Stats {
    pub commands:usize,
    pub drawn_pixels:u64,
    pub clipped_pixels:u64,
    pub triangles:u64,
    pub sampled_texels:u64,
    pub blended_pixels:u64,
}
pub struct Lab {
    width:usize,
    height:usize,
    pixels:Vec<u32>,
    commands:Vec<Command>,
    scissor:(i32,i32,i32,i32),
    stats:Stats,
}
impl Lab {
    pub fn new(width:usize,height:usize)->Result<Self,String>{
        if width==0||height==0||width>LIMIT||height>LIMIT{
            return Err("Dimensoes de GPU fora do limite 1..512".into());
        }
        let n=width.checked_mul(height).ok_or("GPU framebuffer overflow")?;
        Ok(Self{width,height,pixels:vec![0xff000000;n],commands:Vec::new(),
            scissor:(0,0,width as i32,height as i32),stats:Stats::default()})
    }
    pub fn queue(&mut self,command:Command)->Result<(),String>{
        if self.commands.len()>=MAX_COMMANDS{return Err("GPU command buffer overflow".into())}
        self.commands.push(command);Ok(())
    }
    fn put(&mut self,x:i32,y:i32,color:u32,blend:bool){
        let (left,top,w,h)=self.scissor;
        if x<0||y<0||x>=self.width as i32||y>=self.height as i32
          ||x<left||y<top||x>=left.saturating_add(w)||y>=top.saturating_add(h) {
            self.stats.clipped_pixels+=1;return;
        }
        let idx=y as usize*self.width+x as usize;
        if !blend {self.pixels[idx]=color|0xff000000;}
        else {
            let alpha=(color>>24)&255;
            let inverse=255-alpha;
            let old=self.pixels[idx];
            let cr=(color>>16)&255;let cg=(color>>8)&255;let cb=color&255;
            let dr=(old>>16)&255;let dg=(old>>8)&255;let db=old&255;
            let r=(cr*alpha+dr*inverse+127)/255;
            let g=(cg*alpha+dg*inverse+127)/255;
            let b=(cb*alpha+db*inverse+127)/255;
            self.pixels[idx]=0xff000000|(r<<16)|(g<<8)|b;
            self.stats.blended_pixels+=1;
        }
        self.stats.drawn_pixels+=1;
    }
    fn rect(&mut self,x:i32,y:i32,w:i32,h:i32,c:u32,blend:bool){
        if w<=0||h<=0{return;}
        let min_x=x.max(0);let min_y=y.max(0);
        let max_x=x.saturating_add(w).min(self.width as i32);
        let max_y=y.saturating_add(h).min(self.height as i32);
        for py in min_y..max_y{
            for px in min_x..max_x{self.put(px,py,c,blend);}
        }
    }
    fn edge(a:(i32,i32),b:(i32,i32),p:(i32,i32))->i64{
        let vx=(b.0-a.0) as i64;
        let vy=(b.1-a.1) as i64;
        vx*(p.1-a.1) as i64-vy*(p.0-a.0) as i64
    }
    fn triangle(&mut self,a:(i32,i32),b:(i32,i32),c:(i32,i32),color:u32){
        self.stats.triangles+=1;
        let area=Self::edge(a,b,c);
        if area==0{return;}
        let left=a.0.min(b.0).min(c.0).max(0);
        let top=a.1.min(b.1).min(c.1).max(0);
        let right=a.0.max(b.0).max(c.0).min(self.width as i32-1);
        let bottom=a.1.max(b.1).max(c.1).min(self.height as i32-1);
        if left>right||top>bottom{return;}
        for y in top..=bottom {
            for x in left..=right {
                let p=(x,y);
                let e0=Self::edge(a,b,p);
                let e1=Self::edge(b,c,p);
                let e2=Self::edge(c,a,p);
                if (area>0&&e0>=0&&e1>=0&&e2>=0)
                    ||(area<0&&e0<=0&&e1<=0&&e2<=0) {
                    self.put(x,y,color,false);
                }
            }
        }
    }
    fn texture(&mut self,x:i32,y:i32,w:i32,h:i32){
        if w<=0||h<=0{return;}
        let t=[0xff58db7d,0xff113b30,0xfff2c14e,0xffe7f7f0];
        for py in y.max(0)..y.saturating_add(h).min(self.height as i32){
            for px in x.max(0)..x.saturating_add(w).min(self.width as i32){
                let tx=((px-x)*8/w).rem_euclid(2) as usize;
                let ty=((py-y)*8/h).rem_euclid(2) as usize;
                self.put(px,py,t[ty*2+tx],false);
                self.stats.sampled_texels+=1;
            }
        }
    }
    pub fn execute(&mut self){
        let commands=std::mem::take(&mut self.commands);
        self.stats=Stats::default();
        self.stats.commands=commands.len();
        for cmd in commands {
            match cmd {
                Command::Clear(color)=>{
                    self.pixels.fill(color|0xff000000);
                    self.scissor=(0,0,self.width as i32,self.height as i32);
                }
                Command::Scissor{x,y,w,h}=>{
                    self.scissor=(x,y,w.max(0),h.max(0));
                }
                Command::Rect{x,y,w,h,color}=>self.rect(x,y,w,h,color,false),
                Command::Blend{x,y,w,h,color}=>self.rect(x,y,w,h,color,true),
                Command::Triangle{a,b,c,color}=>self.triangle(a,b,c,color),
                Command::Texture{x,y,w,h}=>self.texture(x,y,w,h),
            }
        }
    }
    pub fn pixels(&self)->&[u32]{&self.pixels}
    pub fn stats(&self)->&Stats{&self.stats}
    pub fn summary(&self)->String {
        format!("GPU Xenos LAB (nao e Xenos real)\nFramebuffer: {}x{} ARGB\nComandos: {}\nTriangulos: {}\nPixels desenhados: {}\nPixels com alpha blend: {}\nAmostras de textura original: {}\nPixels cortados por scissor: {}\nBackend: rasterizacao experimental em Rust\nVisualizacao: Android Bitmap\nSEM interpretador de comandos Xenos ou shaders Xbox 360",
            self.width,self.height,self.stats.commands,self.stats.triangles,
            self.stats.drawn_pixels,self.stats.blended_pixels,
            self.stats.sampled_texels,self.stats.clipped_pixels)
    }
}

pub fn demo(mode:i32)->Result<Lab,String>{
    let mut gpu=Lab::new(320,180)?;
    gpu.queue(Command::Clear(0xff06120d))?;
    // Draw a short retro-styled horizon/track via bounded commands.
    gpu.queue(Command::Rect{x:0,y:119,w:320,h:61,color:0xff0d2e1d})?;
    for row in 0..6 {
        let y=121+row*10;
        gpu.queue(Command::Rect{x:0,y,w:320,h:1,color:0xff176642})?;
    }
    for col in 0..11 {
        gpu.queue(Command::Rect{x:col*32,y:119,w:1,h:61,color:0xff154831})?;
    }
    gpu.queue(Command::Scissor{x:12,y:9,w:296,h:164})?;
    match mode.rem_euclid(3){
      0=>{
          gpu.queue(Command::Triangle{a:(160,15),b:(97,119),c:(223,119),color:0xff33d66c})?;
          gpu.queue(Command::Triangle{a:(160,43),b:(122,111),c:(198,111),color:0xff0b3c27})?;
          gpu.queue(Command::Texture{x:20,y:25,w:54,h:54})?;
          gpu.queue(Command::Blend{x:78,y:87,w:160,h:26,color:0x9956baff})?;
      }
      1=>{
          for n in 0..8{
              let x=26+n*34;
              gpu.queue(Command::Triangle{
                  a:(x,30),b:(x-9,97),c:(x+23,104),
                  color:if n%2==0 {0xff68d8fa} else {0xfff2c14e}
              })?;
          }
          gpu.queue(Command::Texture{x:105,y:115,w:110,h:40})?;
          gpu.queue(Command::Blend{x:35,y:72,w:245,h:19,color:0x80f20d66})?;
      }
      _=>{
          gpu.queue(Command::Texture{x:25,y:17,w:270,h:113})?;
          for i in 0..5{
              gpu.queue(Command::Blend{x:40+i*25,y:33+i*12,w:100,h:58,color:0x8058db7d})?;
          }
          gpu.queue(Command::Triangle{a:(156,42),b:(212,115),c:(104,115),color:0xff113b30})?;
      }
    }
    gpu.execute();
    Ok(gpu)
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test] fn command_pipeline_draws_triangle(){
        let mut lab=Lab::new(32,32).unwrap();
        lab.queue(Command::Clear(0xff000000)).unwrap();
        lab.queue(Command::Triangle{a:(4,4),b:(24,4),c:(4,24),color:0xff00ff00}).unwrap();
        lab.execute();
        assert_eq!(lab.pixels()[8*32+8],0xff00ff00);
        assert_eq!(lab.stats().triangles,1);
    }
    #[test] fn scissors_and_blending(){
        let mut lab=Lab::new(8,8).unwrap();
        lab.queue(Command::Clear(0xff000000)).unwrap();
        lab.queue(Command::Scissor{x:0,y:0,w:4,h:4}).unwrap();
        lab.queue(Command::Blend{x:1,y:1,w:7,h:7,color:0x80ff0000}).unwrap();
        lab.execute();
        assert_eq!(lab.pixels()[7*8+7],0xff000000);
        let p=lab.pixels()[1*8+1];
        assert!((p>>16)&255>=126 && (p>>16)&255<=129);
        assert_eq!(lab.stats().blended_pixels,9);
    }
    #[test] fn command_and_dimensions_bounded(){
        assert!(Lab::new(999,10).is_err());
        let mut l=Lab::new(4,4).unwrap();
        for _ in 0..MAX_COMMANDS {l.queue(Command::Clear(0)).unwrap();}
        assert!(l.queue(Command::Clear(0)).is_err());
    }
    #[test] fn demo_renders_all_three_modes(){
        for i in 0..3{
            let l=demo(i).unwrap();
            assert_eq!(l.pixels().len(),320*180);
            assert!(l.stats().drawn_pixels>50);
            assert!(l.summary().contains("GPU Xenos LAB"));
        }
    }
}
