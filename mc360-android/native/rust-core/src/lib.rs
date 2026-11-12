//! Experimental file header reader; NOT an Xbox 360 emulator.
#[no_mangle]
pub unsafe extern "C" fn mc360_probe(ptr: *const u8, len: usize) -> u32 {
    if ptr.is_null() || len < 4 { return 0 }
    let header = core::slice::from_raw_parts(ptr, 4);
    match header {
        b"XEX2" => 1,
        b"CON " | b"LIVE" | b"PIRS" => 2,
        b"PK\x03\x04" => 3,
        _ => 0
    }
}
#[no_mangle]
pub extern "C" fn mc360_height(x: i32, z: i32) -> i32 {
    // Fully original deterministic checkerboard terrain demo.
    1 + ((x.wrapping_mul(31) ^ z.wrapping_mul(17)).unsigned_abs() % 3) as i32
}
#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn test_magic() {
    assert_eq!(unsafe { mc360_probe(b"XEX2".as_ptr(), 4) }, 1);
    assert_eq!(unsafe { mc360_probe(b"CON ".as_ptr(), 4) }, 2);
    assert_eq!(unsafe { mc360_probe(b"HIXX".as_ptr(), 4) }, 0);
  }
}
