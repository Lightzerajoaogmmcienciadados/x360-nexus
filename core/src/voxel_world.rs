//! Nexus Blocks: original voxel sandbox demonstration. NOT Minecraft 360.
use jni::{JNIEnv,objects::{JObject,JByteArray},sys::{jbyteArray,jfloatArray,jint,jboolean,jstring}};
use std::sync::{Mutex,OnceLock};
pub const W:usize=24;
pub const H:usize=12;
pub const D:usize=24;
pub const CELLS:usize=W*H*D;
const SAVE_LEN:usize=30+CELLS;

#[derive(Clone,Debug)]
pub struct World {
    pub cells:Vec<u8>,
    pub seed:u32,
    pub pos:[f32;3],
    pub yaw:f32,
    pub pitch:f32,
    pub selected:u8,
    pub mined:u32,
    pub placed:u32,
}
impl World{
    pub fn index(x:usize,y:usize,z:usize)->usize{x+W*(z+D*y)}
    fn hash(seed:u32,x:u32,z:u32)->u32{
        let mut k=seed^x.wrapping_mul(0x85ebca6b)^z.wrapping_mul(0xc2b2ae35);
        k^=k>>16;k=k.wrapping_mul(0x7feb352d);
        k^=k>>15;k=k.wrapping_mul(0x846ca68b);k^(k>>16)
    }
    pub fn new(seed:u32)->Self{
        let mut cells=vec![0;CELLS];
        for z in 0..D {for x in 0..W{
            let h=2+(Self::hash(seed,x as u32,z as u32)%4) as usize;
            for y in 0..=h {
                cells[Self::index(x,y,z)]=if y==0{3}else if y<h{2}else{1};
            }
        }}
        for z in 3..D-3 {for x in 3..W-3{
            if x.abs_diff(12)<4 && z.abs_diff(12)<4{continue;}
            if Self::hash(seed^0x8765_4321,x as u32,z as u32)%79!=0{continue;}
            let Some(ground)=(1..H-4).rev().find(|&y|cells[Self::index(x,y,z)]==1)
                else{continue;};
            for y in ground+1..=ground+3{cells[Self::index(x,y,z)]=4;}
            for y in ground+3..=ground+5{
                if y>=H{continue;}
                for dz in -2i32..=2{for dx in -2i32..=2{
                    if dx*dx+dz*dz>5{continue;}
                    let nx=(x as i32+dx) as usize;
                    let nz=(z as i32+dz) as usize;
                    let idx=Self::index(nx,y,nz);
                    if cells[idx]==0{cells[idx]=5;}
                }}
            }
        }}
        Self{cells,seed,pos:[12.0,7.0,12.0],yaw:0.75,pitch:-0.38,
            selected:4,mined:0,placed:0}
    }
    fn inside(x:i32,y:i32,z:i32)->bool{
        x>=0&&y>=0&&z>=0&&x<W as i32&&y<H as i32&&z<D as i32
    }
    fn block(&self,x:i32,y:i32,z:i32)->u8{
        if !Self::inside(x,y,z){0}
        else{self.cells[Self::index(x as usize,y as usize,z as usize)]}
    }
    pub fn tick(&mut self,mask:i32,millis:i32){
        let dt=millis.clamp(0,50) as f32/1000.0;
        let axis=|p:i32,n:i32|{
            (if mask&p!=0{1.0f32}else{0.0})-
            (if mask&n!=0{1.0f32}else{0.0})
        };
        self.yaw+=axis(32,16)*1.7*dt;
        self.pitch=(self.pitch+axis(64,128)*1.3*dt).clamp(-1.3,1.3);
        let (s,c)=self.yaw.sin_cos();
        let forward=axis(1,2);let strafe=axis(8,4);
        let speed=if forward!=0.0&&strafe!=0.0{2.35}else{3.3};
        self.pos[0]=(self.pos[0]+(s*forward+c*strafe)*speed*dt)
            .clamp(0.6,W as f32-0.6);
        self.pos[2]=(self.pos[2]+(c*forward-s*strafe)*speed*dt)
            .clamp(0.6,D as f32-0.6);
        self.pos[1]=(self.pos[1]+axis(256,512)*2.7*dt)
            .clamp(1.2,H as f32-0.6);
    }
    pub fn interact(&mut self,place:bool)->bool{
        let cp=self.pitch.cos();
        let dir=[self.yaw.sin()*cp,self.pitch.sin(),self.yaw.cos()*cp];
        let mut previous=None;
        let mut prior=None;
        for t in 1..91{
            let distance=t as f32*0.065;
            let x=(self.pos[0]+dir[0]*distance).floor() as i32;
            let y=(self.pos[1]+dir[1]*distance).floor() as i32;
            let z=(self.pos[2]+dir[2]*distance).floor() as i32;
            if !Self::inside(x,y,z){break;}
            if prior==Some((x,y,z)){continue;}
            prior=Some((x,y,z));
            if self.block(x,y,z)!=0{
                if place {
                    if let Some((a,b,c))=previous{
                        if b>0&&self.block(a,b,c)==0{
                            let idx=Self::index(a as usize,b as usize,c as usize);
                            self.cells[idx]=self.selected;self.placed+=1;
                            return true;
                        }
                    }
                } else if y>0 {
                    let idx=Self::index(x as usize,y as usize,z as usize);
                    self.cells[idx]=0;self.mined+=1;return true;
                }
                return false;
            }
            previous=Some((x,y,z));
        }
        false
    }
    pub fn camera(&self)->[f32;5]{
        [self.pos[0],self.pos[1],self.pos[2],self.yaw,self.pitch]
    }
    pub fn save(&self)->Vec<u8>{
        let mut out=Vec::with_capacity(SAVE_LEN);
        out.extend_from_slice(b"NVOX");out.push(1);
        out.extend_from_slice(&self.seed.to_le_bytes());
        for value in self.camera(){out.extend_from_slice(&value.to_le_bytes());}
        out.push(self.selected);
        out.extend_from_slice(&self.cells);
        out
    }
    pub fn load(bytes:&[u8])->Result<Self,&'static str>{
        if bytes.len()!=SAVE_LEN||&bytes[..4]!=b"NVOX"||bytes[4]!=1{
            return Err("Invalid voxel save");
        }
        let seed=u32::from_le_bytes(bytes[5..9].try_into().unwrap());
        let mut camera=[0.0f32;5];
        for (i,c) in camera.iter_mut().enumerate(){
            let start=9+i*4;
            *c=f32::from_le_bytes(bytes[start..start+4].try_into().unwrap());
        }
        if camera.iter().any(|x|!x.is_finite())
            || camera[0]<0.5||camera[0]>W as f32-0.5
            || camera[1]<1.0||camera[1]>H as f32-0.5
            || camera[2]<0.5||camera[2]>D as f32-0.5
            ||camera[4].abs()>1.31||!(1..=6).contains(&bytes[29])
            ||bytes[30..].iter().any(|&x|x>6){
            return Err("Invalid voxel save data");
        }
        Ok(Self{cells:bytes[30..].to_vec(),seed,
            pos:[camera[0],camera[1],camera[2]],yaw:camera[3],
            pitch:camera[4],selected:bytes[29],mined:0,placed:0})
    }
    pub fn info(&self)->String{
        format!("Nexus Blocks (original Rust + C++)\nMundo: {}x{}x{} • Seed: {}\nBlocos removidos: {} • colocados: {}\nBloco ativo: {} • posição: {:.1}, {:.1}, {:.1}\nNão é Minecraft Xbox 360 Edition.",
            W,H,D,self.seed,self.mined,self.placed,self.selected,
            self.pos[0],self.pos[1],self.pos[2])
    }
}
static INSTANCE:OnceLock<Mutex<World>>=OnceLock::new();
fn state()->&'static Mutex<World>{
    INSTANCE.get_or_init(||Mutex::new(World::new(2026)))
}
#[no_mangle]pub extern "system" fn Java_com_x360nexus_MainActivity_nativeVoxelReset(
    _env:JNIEnv,_:JObject,seed:jint){
    if let Ok(mut w)=state().lock(){*w=World::new(seed as u32);}
}
#[no_mangle]pub extern "system" fn Java_com_x360nexus_MainActivity_nativeVoxelTick(
    _env:JNIEnv,_:JObject,buttons:jint,millis:jint){
    if let Ok(mut w)=state().lock(){w.tick(buttons,millis);}
}
#[no_mangle]pub extern "system" fn Java_com_x360nexus_MainActivity_nativeVoxelAction(
    _env:JNIEnv,_:JObject,action:jint)->jboolean{
    state().lock().map(|mut w|if w.interact(action==1){1}else{0}).unwrap_or(0)
}
#[no_mangle]pub extern "system" fn Java_com_x360nexus_MainActivity_nativeVoxelSelect(
    _env:JNIEnv,_:JObject,id:jint)->jboolean{
    state().lock().map(|mut w|{
        if (1..=6).contains(&id){w.selected=id as u8;1}else{0}
    }).unwrap_or(0)
}
#[no_mangle]pub extern "system" fn Java_com_x360nexus_MainActivity_nativeVoxelSnapshot(
    env:JNIEnv,_:JObject)->jbyteArray{
    let Ok(w)=state().lock() else{return std::ptr::null_mut()};
    env.byte_array_from_slice(&w.cells).map(|v|v.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
#[no_mangle]pub extern "system" fn Java_com_x360nexus_MainActivity_nativeVoxelCamera(
    mut env:JNIEnv,_:JObject)->jfloatArray{
    let Ok(w)=state().lock() else{return std::ptr::null_mut()};
    let Ok(out)=env.new_float_array(5) else{return std::ptr::null_mut()};
    if env.set_float_array_region(&out,0,&w.camera()).is_err(){
        return std::ptr::null_mut();
    }
    out.into_raw()
}
#[no_mangle]pub extern "system" fn Java_com_x360nexus_MainActivity_nativeVoxelSave(
    env:JNIEnv,_:JObject)->jbyteArray{
    let Ok(w)=state().lock() else{return std::ptr::null_mut()};
    env.byte_array_from_slice(&w.save()).map(|v|v.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
#[no_mangle]pub extern "system" fn Java_com_x360nexus_MainActivity_nativeVoxelLoad(
    mut env:JNIEnv,_:JObject,data:JByteArray)->jboolean{
    let Ok(bytes)=env.convert_byte_array(data) else{return 0};
    let Ok(parsed)=World::load(&bytes) else{return 0};
    if let Ok(mut w)=state().lock(){*w=parsed;1}else{0}
}
#[no_mangle]pub extern "system" fn Java_com_x360nexus_MainActivity_nativeVoxelInfo(
    mut env:JNIEnv,_:JObject)->jstring{
    let msg=state().lock().map(|w|w.info()).unwrap_or("World unavailable".into());
    env.new_string(msg).map(|s|s.into_raw()).unwrap_or(std::ptr::null_mut())
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn procedural_world_and_movement(){
        let mut w=World::new(123);
        assert_eq!(w.cells.len(),CELLS);
        assert_eq!(w.cells,World::new(123).cells);
        let start=w.pos;
        w.tick(1|8,50);
        assert_ne!(start,w.pos);
    }
    #[test]fn save_roundtrip_and_corrupt_reject(){
        let mut w=World::new(5);w.selected=2;
        let save=w.save();assert_eq!(save.len(),SAVE_LEN);
        let copy=World::load(&save).unwrap();
        assert_eq!(copy.cells,w.cells);
        let mut corrupt=save.clone();corrupt[45]=255;
        assert!(World::load(&corrupt).is_err());
    }
    #[test]fn preserves_world_bounds(){
        let mut w=World::new(999);
        for _ in 0..1500{w.tick(1|256,100);}
        assert!(w.pos[0]>0.0&&w.pos[1]<H as f32);
    }
}
