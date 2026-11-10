use std::collections::VecDeque;
/// PCM stereo buffer for host output; NOT Xbox 360 XAudio emulation.
#[derive(Debug)]pub struct Audio {samples:VecDeque<i16>,capacity:usize,sample_rate:u32}
impl Audio {
 pub fn new(sample_rate:u32,capacity:usize)->Self{Self{samples:VecDeque::new(),capacity,sample_rate}}
 pub fn sample_rate(&self)->u32{self.sample_rate}
 pub fn queued(&self)->usize{self.samples.len()}
 pub fn push(&mut self,frames:&[i16])->Result<(),&'static str>{if frames.len()>self.capacity.saturating_sub(self.samples.len()){return Err("audio buffer full")}self.samples.extend(frames);Ok(())}
 pub fn pop(&mut self)->Option<i16>{self.samples.pop_front()}
}
#[cfg(test)]mod tests{use super::*;#[test]fn buffer(){let mut a=Audio::new(48000,4);a.push(&[1,2]).unwrap();assert_eq!(a.pop(),Some(1));assert_eq!(a.sample_rate(),48000);}}
