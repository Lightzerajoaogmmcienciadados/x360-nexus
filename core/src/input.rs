#[derive(Debug,Default,Clone,Copy)]pub struct Controller{pub buttons:u32,pub left_x:i16,pub left_y:i16,pub right_x:i16,pub right_y:i16,pub left_trigger:u8,pub right_trigger:u8}
impl Controller{pub fn pressed(&self,mask:u32)->bool{self.buttons&mask!=0}}
#[cfg(test)]mod tests{use super::*;#[test]fn buttons(){let c=Controller{buttons:0b101,..Default::default()};assert!(c.pressed(1));assert!(!c.pressed(2));}}
