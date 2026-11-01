use application::error::{ProcessingError as Error, Result};

struct Bits {
    data: Vec<u8>,
    offset: usize,
}
impl Bits {
    fn new(nal: &[u8]) -> Result<Self> {
        if nal.len() < 3 || (nal[0] >> 1) & 63 != 33 {
            return Err(Error::InvalidImage);
        }
        let mut data = Vec::with_capacity(nal.len() - 2);
        let mut zeros = 0;
        for &byte in &nal[2..] {
            if zeros >= 2 && byte == 3 {
                zeros = 0;
                continue;
            }
            data.push(byte);
            zeros = if byte == 0 { zeros + 1 } else { 0 };
        }
        Ok(Self { data, offset: 0 })
    }
    fn read(&mut self, count: usize) -> Result<u32> {
        if count > 32
            || self
                .offset
                .checked_add(count)
                .is_none_or(|end| end > self.data.len() * 8)
        {
            return Err(Error::InvalidImage);
        }
        let mut value = 0;
        for _ in 0..count {
            value =
                (value << 1) | u32::from((self.data[self.offset / 8] >> (7 - self.offset % 8)) & 1);
            self.offset += 1;
        }
        Ok(value)
    }
    fn skip(&mut self, count: usize) -> Result<()> {
        if self
            .offset
            .checked_add(count)
            .is_none_or(|end| end > self.data.len() * 8)
        {
            return Err(Error::InvalidImage);
        }
        self.offset += count;
        Ok(())
    }
    fn ue(&mut self) -> Result<u32> {
        let mut zeros = 0;
        while self.read(1)? == 0 {
            zeros += 1;
            if zeros > 31 {
                return Err(Error::InvalidImage);
            }
        }
        (1u32 << zeros)
            .checked_sub(1)
            .and_then(|prefix| prefix.checked_add(self.read(zeros).ok()?))
            .ok_or(Error::InvalidImage)
    }
}

/// Read every SPS's allocation geometry, before its conformance crop is applied.
pub(crate) fn coded_size(nal: &[u8]) -> Result<(u32, u32)> {
    let mut bits = Bits::new(nal)?;
    bits.skip(4)?;
    let layers = bits.read(3)? as usize;
    bits.skip(1 + 96)?;
    let mut profiles = [false; 7];
    let mut levels = [false; 7];
    for layer in 0..layers {
        profiles[layer] = bits.read(1)? != 0;
        levels[layer] = bits.read(1)? != 0;
    }
    if layers > 0 {
        bits.skip((8 - layers) * 2)?;
    }
    for layer in 0..layers {
        if profiles[layer] {
            bits.skip(88)?;
        }
        if levels[layer] {
            bits.skip(8)?;
        }
    }
    bits.ue()?;
    let chroma = bits.ue()?;
    if chroma > 3 {
        return Err(Error::InvalidImage);
    }
    if chroma == 3 {
        bits.skip(1)?;
    }
    let width = bits.ue()?;
    let height = bits.ue()?;
    if width == 0 || height == 0 {
        return Err(Error::InvalidImage);
    }
    Ok((width, height))
}
