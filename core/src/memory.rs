#[derive(Debug)]
pub struct Memory {
    data: Vec<u8>,
}

impl Memory {
    pub fn new(bytes: usize) -> Self {
        Self {
            data: vec![0; bytes],
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn read(
        &self,
        addr: usize,
        len: usize,
    ) -> Result<&[u8], &'static str> {
        let end = addr.checked_add(len)
            .ok_or("address overflow")?;
        self.data.get(addr..end)
            .ok_or("memory out of range")
    }

    pub fn write(
        &mut self,
        addr: usize,
        src: &[u8],
    ) -> Result<(), &'static str> {
        let end = addr.checked_add(src.len())
            .ok_or("address overflow")?;
        let dst = self.data.get_mut(addr..end)
            .ok_or("memory out of range")?;
        dst.copy_from_slice(src);
        Ok(())
    }

    pub fn read_u8(
        &self,
        addr: usize,
    ) -> Result<u8, &'static str> {
        Ok(self.read(addr, 1)?[0])
    }

    pub fn write_u8(
        &mut self,
        addr: usize,
        value: u8,
    ) -> Result<(), &'static str> {
        self.write(addr, &[value])
    }

    pub fn read_u32_be(
        &self,
        addr: usize,
    ) -> Result<u32, &'static str> {
        let bytes: [u8; 4] = self.read(addr, 4)?
            .try_into()
            .map_err(|_| "invalid word")?;
        Ok(u32::from_be_bytes(bytes))
    }

    pub fn write_u32_be(
        &mut self,
        addr: usize,
        value: u32,
    ) -> Result<(), &'static str> {
        self.write(addr, &value.to_be_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_endian() {
        let mut mem = Memory::new(16);
        mem.write_u32_be(4, 0x12345678).unwrap();
        assert_eq!(
            mem.read_u32_be(4).unwrap(),
            0x12345678
        );
        assert_eq!(mem.read_u8(4).unwrap(), 0x12);
    }

    #[test]
    fn bounds() {
        let mut mem = Memory::new(8);
        assert!(mem.write(7, &[1, 2]).is_err());
        assert!(mem.read_u32_be(6).is_err());
        assert!(mem.read(usize::MAX, 2).is_err());
    }
}
