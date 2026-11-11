//! XEX2 runtime readiness diagnostics. Never executes or decrypts commercial XEX files.
use crate::xex_loader::{parse, XexReport};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeBlocker {
    MissingEntryPoint, MissingImageBase, EntryPointUnmapped,
    EncryptedOrCompressedImage, XboxKernelImports, XenosGpuUnavailable,
}

#[derive(Debug)]
pub struct RuntimeReport {
    pub entry_point: Option<u32>,
    pub image_base: Option<u32>,
    pub has_import_descriptor: bool,
    pub blockers: Vec<RuntimeBlocker>,
}

pub fn inspect_header(header: &[u8]) -> Result<RuntimeReport, String> {
    let report: XexReport = parse(header)?;
    let has_import_descriptor = report.optional_headers.iter().any(|h| h.key == 0x000401FF);
    let mut blockers = Vec::new();
    if report.entry_point.is_none() { blockers.push(RuntimeBlocker::MissingEntryPoint); }
    if report.image_base.is_none() { blockers.push(RuntimeBlocker::MissingImageBase); }
    // Metadata alone cannot prove the actual code is mapped or executable.
    blockers.push(RuntimeBlocker::EntryPointUnmapped);
    blockers.push(RuntimeBlocker::EncryptedOrCompressedImage);
    blockers.push(RuntimeBlocker::XboxKernelImports);
    blockers.push(RuntimeBlocker::XenosGpuUnavailable);
    Ok(RuntimeReport {entry_point:report.entry_point,image_base:report.image_base,has_import_descriptor,blockers})
}

pub fn diagnostic(header: &[u8]) -> Result<String,String> {
    let r=inspect_header(header)?;
    Ok(format!("XEX Runtime 4.8: header OK\nEntry point (declarado): {}\nImage base (declarada): {}\nImport descriptor: {}\nBloqueios de execucao: {}\nMinecraft Xbox 360: ainda nao executavel",
        r.entry_point.map(|x|format!("0x{x:08X}")).unwrap_or_else(||"ausente".into()),
        r.image_base.map(|x|format!("0x{x:08X}")).unwrap_or_else(||"ausente".into()),
        if r.has_import_descriptor {"presente"} else {"nao localizado"},
        r.blockers.iter().map(|b|format!("{b:?}")).collect::<Vec<_>>().join(", ")))
}

pub fn demo() -> Result<String,String> {
    // Construct a small, synthetic, unencrypted XEX2 header for diagnostic testing.
    let mut h=vec![0u8;80];h[0..4].copy_from_slice(b"XEX2");
    h[8..12].copy_from_slice(&80u32.to_be_bytes());
    h[16..20].copy_from_slice(&64u32.to_be_bytes());
    h[20..24].copy_from_slice(&3u32.to_be_bytes());
    let pairs=[(0x00040006u32,0x000005D0u32),(0x00030000,0x10000400),(0x000401FF,0x00000040)];
    for (i,(key,val)) in pairs.iter().enumerate(){let p=24+i*8;h[p..p+4].copy_from_slice(&key.to_be_bytes());h[p+4..p+8].copy_from_slice(&val.to_be_bytes());}
    diagnostic(&h)
}
#[cfg(test)]mod tests {use super::*;
    #[test]fn synthetic_header(){let d=demo().unwrap();assert!(d.contains("0x000005D0"));assert!(d.contains("nao executavel"));}
    #[test]fn malformed_header(){assert!(inspect_header(b"invalid").is_err());}
}
