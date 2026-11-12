use std::collections::VecDeque;
#[derive(Debug)]pub struct Audio{samples:VecDeque<i16>,capacity:usize,sample_rate:u32}
impl Audio{pub fn new(sample_rate:u32,capacity:usize)->Self{Self{samples:VecDeque::new(),capacity,sample_rate}}pub fn sample_rate(&self)->u32{self.sample_rate}pub fn queued(&self)->usize{self.samples.len()}pub fn push(&mut self,frames:&[i16])->Result<(),&'static str>{if frames.len()>self.capacity.saturating_sub(self.samples.len()){return Err("audio buffer full")}self.samples.extend(frames);Ok(())}pub fn pop(&mut self)->Option<i16>{self.samples.pop_front()}}

#[derive(Debug,Clone,Copy)]
pub struct StereoFrame{pub left:i16,pub right:i16}
/// Original volume-controlled PCM mix; no Xbox 360 XMA audio decoding.
pub fn mix_stereo(a:&[StereoFrame],b:&[StereoFrame],gain_a:f32,gain_b:f32)
    ->Result<Vec<StereoFrame>,&'static str>{
    if a.len()!=b.len()||a.len()>192000 {return Err("invalid PCM mixer buffer")}
    if !gain_a.is_finite()||!gain_b.is_finite()
       ||!(0.0..=2.0).contains(&gain_a)||!(0.0..=2.0).contains(&gain_b){
        return Err("invalid PCM gain")
    }
    let scale=|x:i16,y:i16|((x as f32*gain_a+y as f32*gain_b)
        .round().clamp(-32768.0,32767.0)) as i16;
    Ok(a.iter().zip(b.iter()).map(|(l,r)|StereoFrame{
        left:scale(l.left,r.left),right:scale(l.right,r.right)
    }).collect())
}
/// Linear mono resampling for small original diagnostics, NOT XMA/ADPCM decode.
pub fn resample_linear_mono(samples:&[i16],from_hz:u32,to_hz:u32)
    ->Result<Vec<i16>,&'static str>{
    if samples.is_empty()||samples.len()>96000
       || !(8000..=192000).contains(&from_hz)||!(8000..=192000).contains(&to_hz){
        return Err("invalid resampler input")
    }
    let frames=(samples.len() as u64*to_hz as u64/from_hz as u64) as usize;
    if frames>200000{return Err("resampler output too large")}
    let mut out=Vec::with_capacity(frames);
    for index in 0..frames {
        let position=index as f64*from_hz as f64/to_hz as f64;
        let a=position.floor() as usize;
        let a=a.min(samples.len()-1);
        let b=(a+1).min(samples.len()-1);
        let fraction=(position-a as f64).clamp(0.0,1.0);
        let value=samples[a] as f64*(1.0-fraction)+samples[b] as f64*fraction;
        out.push(value.round().clamp(-32768.0,32767.0) as i16);
    }
    Ok(out)
}
#[cfg(test)]mod mixer_tests{
    use super::*;
    #[test]fn mixer_clips_without_overflow(){
        let a=[StereoFrame{left:30000,right:-30000}];
        let b=[StereoFrame{left:30000,right:-30000}];
        let mix=mix_stereo(&a,&b,1.0,1.0).unwrap();
        assert_eq!(mix[0].left,32767);assert_eq!(mix[0].right,-32768);
    }
    #[test]fn linear_resampler_interpolates(){
        let r=resample_linear_mono(&[0,1000,2000,3000],8000,16000).unwrap();
        assert_eq!(r.len(),8);
        assert_eq!(r[1],500);assert_eq!(r[3],1500);
    }
}
