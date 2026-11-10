#[derive(Debug,PartialEq,Eq)]pub enum FileKind{Xex2,Unknown}
pub fn identify(header:&[u8])->FileKind{if header.starts_with(b"XEX2"){FileKind::Xex2}else{FileKind::Unknown}}
/// Header identification only. Does not decrypt, relocate, or run Xbox 360 executables.
#[cfg(test)]mod tests{use super::*;#[test]fn magic(){assert_eq!(identify(b"XEX2hello"),FileKind::Xex2);assert_eq!(identify(b"random"),FileKind::Unknown);}}
