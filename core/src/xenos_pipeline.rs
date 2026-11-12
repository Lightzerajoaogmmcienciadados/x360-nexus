//! Xenos Pipeline v6.5: original bounded software GPU diagnostic.
//! Implements depth buffering, indexed triangles, Gouraud vertex colors,
//! bilinear procedural texture sampling and a simple projected 3D cube.
//! NOT actual Xbox 360 Xenos, its shader ISA, PM4, EDRAM or game rendering.

const MAX_SIDE:usize=512;
const MAX_VERTICES:usize=1024;
const MAX_INDICES:usize=4096;

#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub x:f32, pub y:f32, pub z:f32,
    pub color:[f32;3],
    pub uv:[f32;2],
}
impl Vertex {
    pub fn new(x:f32,y:f32,z:f32,color:[f32;3],uv:[f32;2])->Self{
        Self{x,y,z,color,uv}
    }
}
#[derive(Debug, Default, Clone)]
pub struct Stats {
    pub draw_calls:u32,
    pub indexed_triangles:u32,
    pub submitted_vertices:u32,
    pub shaded_pixels:u64,
    pub depth_rejections:u64,
    pub bilinear_samples:u64,
}
#[derive(Debug)]
pub struct Pipeline {
    width:usize,
    height:usize,
    color:Vec<u32>,
    z:Vec<f32>,
    scissor:(i32,i32,i32,i32),
    stats:Stats,
}
fn clamp_byte(c:f32)->u32{(c.clamp(0.0,1.0)*255.0).round() as u32}
fn rgb(r:f32,g:f32,b:f32)->u32{
    0xff00_0000|(clamp_byte(r)<<16)|(clamp_byte(g)<<8)|clamp_byte(b)
}
fn edge(a:&Vertex,b:&Vertex,x:f32,y:f32)->f32{
    (x-a.x)*(b.y-a.y)-(y-a.y)*(b.x-a.x)
}
fn procedural_texel(x:usize,y:usize)->[f32;3]{
    let p=((x/2)+(y/2))%2;
    if p==0 {
        [0.16+(x as f32)/12.0,0.62+(y as f32)/24.0,0.28]
    } else {
        [0.14,0.35+(y as f32)/14.0,0.85-(x as f32)/22.0]
    }
}
fn sample_bilinear(uv:[f32;2])->[f32;3]{
    let u=uv[0].clamp(0.0,1.0)*7.0;
    let v=uv[1].clamp(0.0,1.0)*7.0;
    let x=u.floor() as usize;let y=v.floor() as usize;
    let x1=(x+1).min(7);let y1=(y+1).min(7);
    let fu=u-x as f32;let fv=v-y as f32;
    let t00=procedural_texel(x,y);
    let t10=procedural_texel(x1,y);
    let t01=procedural_texel(x,y1);
    let t11=procedural_texel(x1,y1);
    let mut out=[0.0;3];
    for ch in 0..3{
        out[ch]=(t00[ch]*(1.0-fu)+t10[ch]*fu)*(1.0-fv)
            +(t01[ch]*(1.0-fu)+t11[ch]*fu)*fv;
    }
    out
}
impl Pipeline {
    pub fn new(width:usize,height:usize)->Result<Self,String>{
        if width==0||height==0||width>MAX_SIDE||height>MAX_SIDE{
            return Err("GPU: dimensao invalida".into());
        }
        let len=width.checked_mul(height).ok_or("GPU: framebuffer overflow")?;
        Ok(Self {
            width,height,color:vec![0xff05110b;len],z:vec![1.0;len],
            scissor:(0,0,width as i32,height as i32),stats:Stats::default()
        })
    }
    pub fn clear(&mut self,c:u32){
        self.color.fill(c|0xff00_0000);
        self.z.fill(1.0);
        self.stats=Stats::default();
        self.scissor=(0,0,self.width as i32,self.height as i32);
    }
    pub fn scissor(&mut self,x:i32,y:i32,width:i32,height:i32){
        self.scissor=(x,y,width.max(0),height.max(0));
    }
    pub fn draw_indexed(&mut self,vertices:&[Vertex],indices:&[u16],textured:bool)
        ->Result<(),String>{
        if vertices.len()>MAX_VERTICES||indices.len()>MAX_INDICES
            ||indices.len()%3!=0 {
            return Err("GPU: limites de malha excedidos".into());
        }
        for vert in vertices {
            if ![vert.x,vert.y,vert.z,vert.color[0],vert.color[1],vert.color[2],
                    vert.uv[0],vert.uv[1]].iter().all(|x|x.is_finite()){
                return Err("GPU: coordenada nao finita".into());
            }
        }
        if indices.iter().any(|&idx|idx as usize>=vertices.len()){
            return Err("GPU: indice de vertice invalido".into());
        }
        self.stats.draw_calls+=1;
        self.stats.submitted_vertices+=vertices.len() as u32;
        for tri in indices.chunks_exact(3) {
            let a=vertices[tri[0] as usize];
            let b=vertices[tri[1] as usize];
            let c=vertices[tri[2] as usize];
            self.raster_triangle(a,b,c,textured);
            self.stats.indexed_triangles+=1;
        }
        Ok(())
    }
    fn raster_triangle(&mut self,a:Vertex,b:Vertex,c:Vertex,textured:bool){
        let area=edge(&a,&b,c.x,c.y);
        if area.abs()<0.0001{return;}
        let left=a.x.min(b.x).min(c.x).floor().max(0.0) as i32;
        let right=a.x.max(b.x).max(c.x).ceil().min(self.width as f32-1.0) as i32;
        let top=a.y.min(b.y).min(c.y).floor().max(0.0) as i32;
        let bottom=a.y.max(b.y).max(c.y).ceil().min(self.height as f32-1.0) as i32;
        let (sx,sy,sw,sh)=self.scissor;
        let lx=left.max(sx.max(0));
        let rx=right.min(sx.saturating_add(sw).saturating_sub(1));
        let ty=top.max(sy.max(0));
        let by=bottom.min(sy.saturating_add(sh).saturating_sub(1));
        if lx>rx||ty>by{return;}
        for py in ty..=by {
            for px in lx..=rx {
                let fx=px as f32+0.5;let fy=py as f32+0.5;
                let w0=edge(&b,&c,fx,fy)/area;
                let w1=edge(&c,&a,fx,fy)/area;
                let w2=1.0-w0-w1;
                if w0 < -0.00001||w1 < -0.00001||w2 < -0.00001{continue;}
                let z=a.z*w0+b.z*w1+c.z*w2;
                if !(0.0..=1.0).contains(&z){continue;}
                let index=py as usize*self.width+px as usize;
                if z>=self.z[index]{
                    self.stats.depth_rejections+=1;
                    continue;
                }
                let mut cr=a.color[0]*w0+b.color[0]*w1+c.color[0]*w2;
                let mut cg=a.color[1]*w0+b.color[1]*w1+c.color[1]*w2;
                let mut cb=a.color[2]*w0+b.color[2]*w1+c.color[2]*w2;
                if textured {
                    let uv=[
                        a.uv[0]*w0+b.uv[0]*w1+c.uv[0]*w2,
                        a.uv[1]*w0+b.uv[1]*w1+c.uv[1]*w2
                    ];
                    let tex=sample_bilinear(uv);
                    cr*=tex[0];cg*=tex[1];cb*=tex[2];
                    self.stats.bilinear_samples+=1;
                }
                self.z[index]=z;
                self.color[index]=rgb(cr,cg,cb);
                self.stats.shaded_pixels+=1;
            }
        }
    }
    pub fn pixels(&self)->&[u32]{&self.color}
    pub fn stats(&self)->&Stats{&self.stats}
    pub fn summary(&self)->String{
        format!("Xenos LAB v6.5 — pipeline experimental\nFramebuffer: {}x{} ARGB + Z-buffer\nDraw calls: {}\nVertices: {}\nTriangulos indexados: {}\nPixels rasterizados: {}\nFragmentos rejeitados pelo Z-test: {}\nAmostras bilineares: {}\nCores: interpolacao por vertice\nRasterizacao: software Rust (CPU ARM64)\nNao interpreta comandos ou shaders da GPU Xenos real.",
            self.width,self.height,self.stats.draw_calls,self.stats.submitted_vertices,
            self.stats.indexed_triangles,self.stats.shaded_pixels,
            self.stats.depth_rejections,self.stats.bilinear_samples)
    }
}
fn v(x:f32,y:f32,z:f32,r:f32,g:f32,b:f32,u:f32,w:f32)->Vertex{
    Vertex::new(x,y,z,[r,g,b],[u,w])
}
fn background(p:&mut Pipeline){
    p.clear(0xff07180f);
    // Draw background with flat procedural diagnostic grid.
    for y in 0..p.height {
        for x in 0..p.width {
            if y%20==0 || x%25==0{
                p.color[y*p.width+x]=0xff153e29;
            }
        }
    }
}
fn quad(p:&mut Pipeline,xy:[[f32;3];4],colors:[[f32;3];4],textured:bool)
    ->Result<(),String>{
    let verts=[
        Vertex::new(xy[0][0],xy[0][1],xy[0][2],colors[0],[0.0,0.0]),
        Vertex::new(xy[1][0],xy[1][1],xy[1][2],colors[1],[1.0,0.0]),
        Vertex::new(xy[2][0],xy[2][1],xy[2][2],colors[2],[1.0,1.0]),
        Vertex::new(xy[3][0],xy[3][1],xy[3][2],colors[3],[0.0,1.0]),
    ];
    p.draw_indexed(&verts,&[0,1,2,0,2,3],textured)
}
pub fn demo(mode:i32)->Result<Pipeline,String>{
    let mut p=Pipeline::new(320,180)?;
    background(&mut p);
    match mode.rem_euclid(3){
      0=>{
        // Layer order deliberately wrong: blue near, warm red far drawn last.
        // Correct Z-buffer keeps the blue front triangle visible.
        let near=[
            v(155.0,19.0,0.17,0.10,0.84,1.0,0.0,0.0),
            v(45.0,153.0,0.17,0.10,0.84,1.0,0.0,0.0),
            v(262.0,150.0,0.17,0.10,0.84,1.0,0.0,0.0)
        ];
        p.draw_indexed(&near,&[0,1,2],false)?;
        let far=[
            v(80.0,35.0,0.72,1.0,0.30,0.14,0.0,0.0),
            v(275.0,37.0,0.72,1.0,0.30,0.14,0.0,0.0),
            v(162.0,169.0,0.72,1.0,0.30,0.14,0.0,0.0)
        ];
        p.draw_indexed(&far,&[0,1,2],false)?;
      }
      1=>{
        // Barycentric/Gouraud shading over indexed mesh.
        let color=[
            v(155.0,12.0,0.5,1.0,0.22,0.24,0.0,0.0),
            v(18.0,162.0,0.5,0.12,1.0,0.32,0.0,0.0),
            v(303.0,157.0,0.5,0.14,0.4,1.0,0.0,0.0),
            v(158.0,110.0,0.35,1.0,0.9,0.16,0.0,0.0)
        ];
        p.draw_indexed(&color,&[0,1,3,1,2,3,2,0,3],false)?;
      }
      _=>{
        // 3D cube with rotation + perspective projection to screen triangles.
        let corners=[
            [-1.0,-1.0,-1.0],[1.0,-1.0,-1.0],
            [1.0,1.0,-1.0],[-1.0,1.0,-1.0],
            [-1.0,-1.0,1.0],[1.0,-1.0,1.0],
            [1.0,1.0,1.0],[-1.0,1.0,1.0]
        ];
        let yaw=0.63_f32;let pitch=-0.36_f32;
        let (sy,cy)=yaw.sin_cos();let (sp,cp)=pitch.sin_cos();
        let mut projected=[[0.0f32;3];8];
        for (i,c) in corners.iter().enumerate(){
            let x=c[0]*cy+c[2]*sy;
            let z=-c[0]*sy+c[2]*cy;
            let y=c[1]*cp-z*sp;
            let z2=c[1]*sp+z*cp;
            let distance=4.0+z2;
            projected[i]=[
                160.0+x*174.0/distance,
                90.0+y*145.0/distance,
                (distance/8.0).clamp(0.0,1.0)
            ];
        }
        let faces=[
          [0,1,2,3],[4,5,6,7],[0,1,5,4],
          [3,2,6,7],[1,2,6,5],[0,3,7,4]
        ];
        for face in faces {
            quad(&mut p,
                [projected[face[0]],projected[face[1]],
                 projected[face[2]],projected[face[3]]],
                [[1.0,1.0,1.0];4],true)?;
        }
      }
    }
    Ok(p)
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn zbuffer_occludes_far_drawn_last(){
        let mut p=Pipeline::new(32,32).unwrap();
        let triangle=|z,color:[f32;3]|[
            Vertex::new(4.0,4.0,z,color,[0.0,0.0]),
            Vertex::new(27.0,4.0,z,color,[1.0,0.0]),
            Vertex::new(4.0,27.0,z,color,[0.0,1.0])
        ];
        p.draw_indexed(&triangle(0.2,[0.0,1.0,0.0]),&[0,1,2],false).unwrap();
        p.draw_indexed(&triangle(0.8,[1.0,0.0,0.0]),&[0,1,2],false).unwrap();
        assert_eq!(p.pixels()[9*32+9],0xff00ff00);
        assert!(p.stats().depth_rejections>0);
    }
    #[test]fn interpolates_three_vertex_colors(){
        let mut p=Pipeline::new(32,32).unwrap();
        p.draw_indexed(&[
            v(1.0,1.0,0.2,1.0,0.0,0.0,0.0,0.0),
            v(28.0,1.0,0.2,0.0,1.0,0.0,0.0,0.0),
            v(1.0,28.0,0.2,0.0,0.0,1.0,0.0,0.0)
        ],&[0,1,2],false).unwrap();
        let c=p.pixels()[8*32+8];
        assert!((c>>16)&255>10 && (c>>8)&255>10 && c&255>10);
    }
    #[test]fn bilinear_middle_blends_neighbors(){
        let t=sample_bilinear([0.22,0.22]);
        let corner=procedural_texel(1,1);
        assert_ne!(t,corner);
        assert!(t.iter().all(|x|x.is_finite()&&*x>=0.0&&*x<=1.0));
    }
    #[test]fn rejects_invalid_mesh(){
        let mut p=Pipeline::new(16,16).unwrap();
        let one=[v(1.0,2.0,0.3,1.0,1.0,1.0,0.0,0.0)];
        assert!(p.draw_indexed(&one,&[0,1,2],true).is_err());
        assert!(p.draw_indexed(&one,&[0,0],true).is_err());
        let bad=[v(f32::NAN,2.0,0.3,1.0,1.0,1.0,0.0,0.0)];
        assert!(p.draw_indexed(&bad,&[0,0,0],true).is_err());
    }
    #[test]fn three_gpu_scenes_produce_pixels(){
        for mode in 0..3 {
            let p=demo(mode).unwrap();
            assert_eq!(p.pixels().len(),320*180);
            assert!(p.stats().shaded_pixels>100);
        }
        let cube=demo(2).unwrap();
        assert!(cube.stats().bilinear_samples>100);
        assert!(cube.stats().indexed_triangles>=12);
    }
}
