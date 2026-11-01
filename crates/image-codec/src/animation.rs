use application::{
    error::{ProcessingError as Error, Result},
    images::{MAX_FRAME_DURATION_MS, MAX_WEBP_DIMENSION, MAX_WEBP_FILE_BYTES},
};

pub struct AnimationMux {
    data: Vec<u8>,
    width: u32,
    height: u32,
    frames: u32,
    maximum: u64,
}

fn le24(value: u32) -> [u8; 3] {
    let bytes = value.to_le_bytes();
    [bytes[0], bytes[1], bytes[2]]
}

impl AnimationMux {
    pub fn new(width: u32, height: u32, maximum: u64) -> Result<Self> {
        if width == 0 || height == 0 || width > MAX_WEBP_DIMENSION || height > MAX_WEBP_DIMENSION {
            return Err(Error::InternalError);
        }
        let maximum = maximum.min(MAX_WEBP_FILE_BYTES).min(isize::MAX as u64);
        if maximum < 44 {
            return Err(Error::OutputLimitsExceeded);
        }
        let mut data = Vec::new();
        data.try_reserve_exact(44)
            .map_err(|_| Error::OutputLimitsExceeded)?;
        data.extend_from_slice(b"RIFF\0\0\0\0WEBPVP8X\x0a\0\0\0\x02\0\0\0");
        data.extend_from_slice(&le24(width - 1));
        data.extend_from_slice(&le24(height - 1));
        data.extend_from_slice(b"ANIM\x06\0\0\0\0\0\0\0\0\0");
        Ok(Self {
            data,
            width,
            height,
            frames: 0,
            maximum,
        })
    }

    pub fn len(&self) -> u64 {
        self.data.len() as u64
    }

    pub fn add(&mut self, encoded: &[u8], delay: u32, remaining: u64) -> Result<()> {
        if delay == 0 || delay > MAX_FRAME_DURATION_MS {
            return Err(Error::UnrepresentableAnimation);
        }
        let mut chunks = Vec::new();
        let mut cursor = 12usize;
        let mut frame_bytes = 24usize;
        let mut alpha = false;
        let mut image_chunks = 0;
        while cursor < encoded.len() {
            let header = encoded
                .get(cursor..cursor + 8)
                .ok_or(Error::InternalError)?;
            let length =
                u32::from_le_bytes(header[4..8].try_into().map_err(|_| Error::InternalError)?)
                    as usize;
            let end = cursor
                .checked_add(8)
                .and_then(|n| n.checked_add(length))
                .and_then(|n| n.checked_add(length % 2))
                .ok_or(Error::OutputLimitsExceeded)?;
            let chunk = encoded.get(cursor..end).ok_or(Error::InternalError)?;
            match &header[..4] {
                b"VP8 " | b"VP8L" | b"ALPH" => {
                    frame_bytes = frame_bytes
                        .checked_add(chunk.len())
                        .ok_or(Error::OutputLimitsExceeded)?;
                    alpha |= &header[..4] == b"ALPH";
                    if &header[..4] == b"VP8L" {
                        alpha |= chunk.get(12).is_some_and(|b| b & 0x10 != 0);
                    }
                    image_chunks += usize::from(&header[..4] != b"ALPH");
                    chunks.push(chunk);
                }
                b"VP8X" => {}
                _ => return Err(Error::InternalError),
            }
            cursor = end;
        }
        if cursor != encoded.len() || image_chunks != 1 {
            return Err(Error::InternalError);
        }
        let total = self
            .len()
            .checked_add(frame_bytes as u64)
            .ok_or(Error::OutputLimitsExceeded)?;
        if frame_bytes as u64 > remaining || total > self.maximum {
            return Err(Error::OutputLimitsExceeded);
        }
        // Keep only encoded subchunks. This also retains a one-frame animation's duration and loop.
        self.data
            .try_reserve_exact(frame_bytes)
            .map_err(|_| Error::OutputLimitsExceeded)?;
        self.data.extend_from_slice(b"ANMF");
        self.data
            .extend_from_slice(&(frame_bytes as u32 - 8).to_le_bytes());
        self.data.extend_from_slice(&[0; 6]);
        self.data.extend_from_slice(&le24(self.width - 1));
        self.data.extend_from_slice(&le24(self.height - 1));
        self.data.extend_from_slice(&le24(delay));
        self.data.push(2); // The encoded image replaces the complete canvas.
        for chunk in chunks {
            self.data.extend_from_slice(chunk);
        }
        if alpha {
            self.data[20] |= 0x10;
        }
        self.frames = self
            .frames
            .checked_add(1)
            .ok_or(Error::ImageLimitsExceeded)?;
        Ok(())
    }

    pub fn finish(mut self, loops: u16) -> Result<Vec<u8>> {
        if self.frames == 0 {
            return Err(Error::InvalidImage);
        }
        let riff_size =
            u32::try_from(self.data.len() - 8).map_err(|_| Error::OutputLimitsExceeded)?;
        self.data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        self.data[42..44].copy_from_slice(&loops.to_le_bytes());
        Ok(self.data)
    }
}
