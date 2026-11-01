//! Just enough PE validation to reject files that are not 32-bit Windows executables.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeError {
    NotPe,
    Not32Bit,
    Truncated,
}

impl std::fmt::Display for PeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PeError::NotPe => "not a Windows executable",
            PeError::Not32Bit => "not a 32-bit Windows executable",
            PeError::Truncated => "the executable is truncated",
        })
    }
}

fn u16_at(bytes: &[u8], offset: usize) -> Result<u16, PeError> {
    let raw = bytes.get(offset..offset + 2).ok_or(PeError::Truncated)?;
    Ok(u16::from_le_bytes([raw[0], raw[1]]))
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32, PeError> {
    let raw = bytes.get(offset..offset + 4).ok_or(PeError::Truncated)?;
    Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

pub fn validate_pe32(bytes: &[u8]) -> Result<(), PeError> {
    if bytes.get(0..2) != Some(b"MZ") {
        return Err(PeError::NotPe);
    }
    let pe = u32_at(bytes, 0x3c)? as usize;
    if bytes.get(pe..pe + 4) != Some(b"PE\0\0") {
        return Err(PeError::NotPe);
    }
    if u16_at(bytes, pe + 24)? != 0x10b {
        return Err(PeError::Not32Bit);
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn synthetic() -> Vec<u8> {
        let mut bytes = vec![0; 0x200];
        bytes[0..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&0x40u32.to_le_bytes());
        bytes[0x40..0x44].copy_from_slice(b"PE\0\0");
        bytes[0x58..0x5a].copy_from_slice(&0x10bu16.to_le_bytes());
        bytes
    }

    #[test]
    fn accepts_pe32_and_rejects_other_files() {
        assert_eq!(validate_pe32(&synthetic()), Ok(()));
        assert_eq!(validate_pe32(b"hello"), Err(PeError::NotPe));
        let mut pe64 = synthetic();
        pe64[0x58..0x5a].copy_from_slice(&0x20bu16.to_le_bytes());
        assert_eq!(validate_pe32(&pe64), Err(PeError::Not32Bit));
        let mut truncated = synthetic();
        truncated.truncate(0x44);
        assert_eq!(validate_pe32(&truncated), Err(PeError::Truncated));
    }
}
