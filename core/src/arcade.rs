//! Nexus Paddle — original tiny game running entirely in the Rust/Android Nexus core.
//!
//! This is a PLAYABLE NATIVE HOMEBREW demonstration with software graphics and
//! touch / physical controller input. It is not an Xbox 360 commercial game,
//! nor does it implement Xenos PM4, VMX128, XEX, XAM or a Xenon MMU.
use jni::{JNIEnv,objects::JObject,sys::{jint,jintArray,jstring}};
use std::sync::{Mutex,OnceLock};

pub const WIDTH:usize=320;
pub const HEIGHT:usize=180;
const PADDLE_W:i32=6;
const PADDLE_H:i32=32;
const LEFT_X:i32=16;
const RIGHT_X:i32=WIDTH as i32-LEFT_X-PADDLE_W;
const BALL:i32=6;
const BLACK:u32=0xff06150d;
const GREEN:u32=0xff48de75;
const DIM:u32=0xff16482a;
const WHITE:u32=0xffedfff1;
const SKY:u32=0xff43aed2;
#[derive(Debug)]
pub struct Arcade {
    paddle:f32, ai:f32,
    ball_x:f32, ball_y:f32,
    velocity_x:f32, velocity_y:f32,
    player:u32, computer:u32,
    frames:u64, running:bool,
    pixels:Vec<u32>,
}
impl Arcade{
    pub fn new()->Self{
        Self{
            paddle:74.0,ai:74.0,ball_x:WIDTH as f32/2.0,
            ball_y:HEIGHT as f32/2.0,velocity_x:115.0,velocity_y:55.0,
            player:0,computer:0,frames:0,running:true,
            pixels:vec![BLACK;WIDTH*HEIGHT],
        }
    }
    pub fn reset(&mut self){*self=Self::new();}
    fn round(&mut self,direction:f32){
        self.ball_x=WIDTH as f32/2.0;
        self.ball_y=HEIGHT as f32/2.0;
        self.velocity_x=115.0*direction;
        self.velocity_y=if (self.player+self.computer)%2==0{68.0}else{-68.0};
    }
    pub fn update(&mut self,buttons:u32,delta_ms:i32){
        let dt=(delta_ms.clamp(0,50) as f32)/1000.0;
        if !self.running{return;}
        let mut direction=0.0f32;
        if buttons&1!=0{direction-=1.0;} // touch UP / D-pad UP
        if buttons&2!=0{direction+=1.0;} // touch DOWN / D-pad DOWN
        self.paddle=(self.paddle+direction*188.0*dt)
            .clamp(5.0,HEIGHT as f32-PADDLE_H as f32-5.0);
        let target=self.ball_y-(PADDLE_H/2) as f32;
        self.ai+=(target-self.ai).clamp(-116.0*dt,116.0*dt);
        self.ai=self.ai.clamp(5.0,HEIGHT as f32-PADDLE_H as f32-5.0);
        self.ball_x+=self.velocity_x*dt;
        self.ball_y+=self.velocity_y*dt;
        if self.ball_y<=8.0{
            self.ball_y=8.0;
            self.velocity_y=self.velocity_y.abs();
        }
        if self.ball_y>=HEIGHT as f32-BALL as f32-8.0{
            self.ball_y=HEIGHT as f32-BALL as f32-8.0;
            self.velocity_y=-self.velocity_y.abs();
        }
        let hit_left=self.ball_x<=LEFT_X as f32+PADDLE_W as f32
            && self.ball_x>=LEFT_X as f32-4.0
            && self.ball_y+BALL as f32>=self.paddle
            && self.ball_y<=self.paddle+PADDLE_H as f32;
        let hit_right=self.ball_x+BALL as f32>=RIGHT_X as f32
            && self.ball_x<RIGHT_X as f32+PADDLE_W as f32+4.0
            && self.ball_y+BALL as f32>=self.ai
            && self.ball_y<=self.ai+PADDLE_H as f32;
        if hit_left && self.velocity_x<0.0{
            self.ball_x=LEFT_X as f32+PADDLE_W as f32;
            self.velocity_x=(-self.velocity_x*1.045).min(245.0).abs();
            self.velocity_y+=((self.ball_y+3.0)-(self.paddle+16.0))*2.0;
        }
        if hit_right && self.velocity_x>0.0{
            self.ball_x=RIGHT_X as f32-BALL as f32;
            self.velocity_x=-(self.velocity_x*1.045).min(245.0);
            self.velocity_y+=((self.ball_y+3.0)-(self.ai+16.0))*2.0;
        }
        self.velocity_y=self.velocity_y.clamp(-210.0,210.0);
        if self.ball_x<0.0{self.computer+=1;self.round(1.0);}
        if self.ball_x>WIDTH as f32{self.player+=1;self.round(-1.0);}
        self.frames+=1;
    }
    fn rect(&mut self,x:i32,y:i32,w:i32,h:i32,color:u32){
        if w<=0||h<=0{return;}
        let x0=x.max(0) as usize;
        let y0=y.max(0) as usize;
        let x1=x.saturating_add(w).min(WIDTH as i32).max(0) as usize;
        let y1=y.saturating_add(h).min(HEIGHT as i32).max(0) as usize;
        for yy in y0..y1{
            for xx in x0..x1{self.pixels[yy*WIDTH+xx]=color;}
        }
    }
    fn number(&mut self,x:i32,y:i32,n:u32,color:u32){
        const DIGITS:[[u8;5];10]=[
            [0b111,0b101,0b101,0b101,0b111],
            [0b010,0b110,0b010,0b010,0b111],
            [0b111,0b001,0b111,0b100,0b111],
            [0b111,0b001,0b111,0b001,0b111],
            [0b101,0b101,0b111,0b001,0b001],
            [0b111,0b100,0b111,0b001,0b111],
            [0b111,0b100,0b111,0b101,0b111],
            [0b111,0b001,0b001,0b001,0b001],
            [0b111,0b101,0b111,0b101,0b111],
            [0b111,0b101,0b111,0b001,0b111],
        ];
        let rows=DIGITS[(n%10) as usize];
        for (yy,row) in rows.iter().enumerate(){
            for xx in 0..3{
                if row&(1<<(2-xx))!=0{self.rect(x+(xx*3),y+(yy as i32)*3,3,3,color);}
            }
        }
    }
    pub fn render(&mut self)->&[u32]{
        self.pixels.fill(BLACK);
        self.rect(0,0,WIDTH as i32,4,GREEN);
        self.rect(0,HEIGHT as i32-4,WIDTH as i32,4,GREEN);
        for i in 0..12{
            self.rect(WIDTH as i32/2-1,i*15+6,2,8,DIM);
        }
        self.rect(LEFT_X,self.paddle as i32,PADDLE_W,PADDLE_H,GREEN);
        self.rect(RIGHT_X,self.ai as i32,PADDLE_W,PADDLE_H,SKY);
        self.rect(self.ball_x as i32,self.ball_y as i32,BALL,BALL,WHITE);
        self.number(130,14,self.player,WHITE);
        self.number(177,14,self.computer,SKY);
        &self.pixels
    }
    pub fn status(&self)->String{
        format!("Nexus Paddle - jogo Rust sem Xenia\nVoce: {}  CPU: {}\nFrames processados: {}\nControle: CIMA / BAIXO (toque ou gamepad)\nMotor: Rust ARM64, framebuffer 320x180, renderizacao propria\nNao executa ISO/XEX comercial do Xbox 360.",self.player,self.computer,self.frames)
    }
}
impl Default for Arcade {fn default()->Self{Self::new()}}
static GAME:OnceLock<Mutex<Arcade>>=OnceLock::new();
fn state()->&'static Mutex<Arcade>{GAME.get_or_init(||Mutex::new(Arcade::new()))}

#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeArcadeReset(
    _env:JNIEnv,_:JObject
){
    if let Ok(mut game)=state().lock(){game.reset();}
}

#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeArcadeFrame(
    mut env:JNIEnv,_:JObject,buttons:jint,millis:jint
)->jintArray {
    let Ok(mut game)=state().lock() else{return std::ptr::null_mut()};
    game.update(buttons as u32,millis);
    let values:Vec<i32>=game.render().iter().map(|&c|c as i32).collect();
    let Ok(array)=env.new_int_array(values.len() as i32) else{return std::ptr::null_mut()};
    if env.set_int_array_region(&array,0,&values).is_err(){return std::ptr::null_mut();}
    array.into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_x360nexus_MainActivity_nativeArcadeStatus(
    mut env:JNIEnv,_:JObject
)->jstring{
    let msg=state().lock().map(|s|s.status()).unwrap_or_else(|_|"Motor ocupado".into());
    env.new_string(msg).map(|s|s.into_raw()).unwrap_or(std::ptr::null_mut())
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn moves_player_and_scores(){
        let mut game=Arcade::new();
        let top=game.paddle;
        game.update(1,20);
        assert!(game.paddle<top);
        game.ball_x=WIDTH as f32+2.0;
        game.update(0,20);
        assert_eq!(game.player,1);
        assert_eq!(game.render().len(),WIDTH*HEIGHT);
    }
    #[test]fn caps_delta_and_bounds(){
        let mut game=Arcade::new();
        for _ in 0..2000{game.update(2,1000);}
        assert!(game.paddle<=HEIGHT as f32-PADDLE_H as f32-5.0);
        assert!(game.status().contains("sem Xenia"));
    }
    #[test]fn reset_clears_score(){
        let mut game=Arcade::new();
        game.player=5;
        game.reset();
        assert_eq!(game.player,0);
    }
}