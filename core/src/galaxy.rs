//! Bounded execution of an original PowerPC test program; not an Xbox game.
use crate::Machine;
pub fn run_demo()->Result<String,String>{
 let mut m=Machine::new();
 // addi r3,r0,42; addi r4,r3,8; addi r5,r0,7
 let code=[0x38,0x60,0,42,0x38,0x83,0,8,0x38,0xA0,0,7];
 m.load_test_program(&code).map_err(str::to_string)?;
 for _ in 0..3{m.step()?}
 Ok(format!("PowerPC: r3={} r4={} r5={} | kernel ticks={}",m.cpu.gpr[3],m.cpu.gpr[4],m.cpu.gpr[5],m.kernel.ticks()))
}
#[cfg(test)]mod tests{#[test]fn demo(){let msg=super::run_demo().unwrap();assert!(msg.contains("r3=42"));assert!(msg.contains("r4=50"));assert!(msg.contains("r5=7"));}}
