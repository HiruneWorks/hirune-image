use application::{
    error::{ProcessingError as Error, Result},
    images::{ImageFacts, Limits},
};
use image::metadata::Orientation;
use libwebp_sys as ffi;

pub struct WebpStructure<'a> {
    pub facts: ImageFacts,
    pub delays: Vec<u32>,
    pub icc: Option<&'a [u8]>,
    pub orientation: Orientation,
    pub metadata: bool,
}
fn le24(data: &[u8]) -> u32 {
    u32::from_le_bytes([data[0], data[1], data[2], 0])
}
pub fn webp_structure<'a>(data: &'a [u8], limits: &Limits) -> Result<WebpStructure<'a>> {
    if data.len() < 12
        || &data[..4] != b"RIFF"
        || &data[8..12] != b"WEBP"
        || u32::from_le_bytes(data[4..8].try_into().map_err(|_| Error::InvalidImage)?) as u64 + 8
            != data.len() as u64
    {
        return Err(Error::InvalidImage);
    }
    let (mut cursor, mut delays, mut loops, mut icc, mut exif, mut metadata) =
        (12, Vec::new(), None, None, None, false);
    let mut dimensions = None;
    let mut scanned_duration = 0u32;
    while cursor < data.len() {
        let header = data.get(cursor..cursor + 8).ok_or(Error::InvalidImage)?;
        let length =
            u32::from_le_bytes(header[4..8].try_into().map_err(|_| Error::InvalidImage)?) as usize;
        let end = cursor
            .checked_add(8)
            .and_then(|n| n.checked_add(length))
            .ok_or(Error::InvalidImage)?;
        let body = data.get(cursor + 8..end).ok_or(Error::InvalidImage)?;
        match &header[..4] {
            b"VP8X" => {
                if body.len() != 10 || dimensions.is_some() {
                    return Err(Error::InvalidImage);
                }
                dimensions = Some((le24(&body[4..7]) + 1, le24(&body[7..10]) + 1));
            }
            b"ANIM" => {
                if body.len() != 6 || loops.is_some() {
                    return Err(Error::InvalidImage);
                }
                loops = Some(u16::from_le_bytes([body[4], body[5]]));
            }
            b"ANMF" => {
                if body.len() < 16 || delays.len() >= limits.frames as usize {
                    return Err(Error::ImageLimitsExceeded);
                }
                let delay = le24(&body[12..15]);
                scanned_duration = scanned_duration
                    .checked_add(delay)
                    .ok_or(Error::ImageLimitsExceeded)?;
                if scanned_duration > limits.duration_ms {
                    return Err(Error::ImageLimitsExceeded);
                }
                delays.push(delay);
            }
            b"ICCP" => {
                metadata = true;
                if icc.is_some() || body.len() > 1 << 20 {
                    return Err(Error::UnsupportedColorProfile);
                }
                icc = Some(body);
            }
            b"EXIF" => {
                metadata = true;
                if exif.is_some() {
                    return Err(Error::InvalidImage);
                }
                exif = Some(body);
            }
            b"XMP " => {
                metadata = true;
            }
            b"VP8 " | b"VP8L" | b"ALPH" => {}
            _ => {
                metadata = true;
            }
        }
        cursor = end.checked_add(length % 2).ok_or(Error::InvalidImage)?;
        if cursor > data.len() {
            return Err(Error::InvalidImage);
        }
    }
    let mut width = 0;
    let mut height = 0;
    if unsafe { ffi::WebPGetInfo(data.as_ptr(), data.len(), &mut width, &mut height) } != 1
        || width <= 0
        || height <= 0
    {
        return Err(Error::InvalidImage);
    }
    if dimensions.is_some_and(|d| d != (width as u32, height as u32)) {
        return Err(Error::InvalidImage);
    }
    let animated = loops.is_some();
    if animated != !delays.is_empty() {
        return Err(Error::InvalidImage);
    }
    let frame_count = if animated { delays.len() as u32 } else { 1 };
    if frame_count == 1 && animated {
        return Err(Error::UnrepresentableAnimation);
    }
    let duration_ms = delays
        .iter()
        .try_fold(0u32, |a, b| a.checked_add(*b))
        .ok_or(Error::ImageLimitsExceeded)?;
    if animated && delays.contains(&0) {
        return Err(Error::UnrepresentableAnimation);
    }
    if !animated {
        delays.push(0)
    }
    Ok(WebpStructure {
        facts: ImageFacts {
            width: width as u32,
            height: height as u32,
            frame_count,
            duration_ms,
            loop_count: loops.unwrap_or(1),
        },
        delays,
        icc,
        orientation: exif
            .and_then(Orientation::from_exif_chunk)
            .unwrap_or(Orientation::NoTransforms),
        metadata,
    })
}

pub struct GifStructure {
    pub facts: ImageFacts,
    pub delays: Vec<u32>,
}
fn skip_subblocks(data: &[u8], cursor: &mut usize) -> Result<()> {
    loop {
        let count = usize::from(*data.get(*cursor).ok_or(Error::InvalidImage)?);
        *cursor += 1;
        *cursor = cursor.checked_add(count).ok_or(Error::InvalidImage)?;
        if *cursor > data.len() {
            return Err(Error::InvalidImage);
        }
        if count == 0 {
            return Ok(());
        }
    }
}
pub fn gif_structure(data: &[u8], limits: &Limits) -> Result<GifStructure> {
    if data.len() < 13 || !matches!(&data[..6], b"GIF87a" | b"GIF89a") {
        return Err(Error::InvalidImage);
    }
    let width = u32::from(u16::from_le_bytes([data[6], data[7]]));
    let height = u32::from(u16::from_le_bytes([data[8], data[9]]));
    let mut cursor = 13
        + if data[10] & 128 != 0 {
            3 * (1usize << ((data[10] & 7) + 1))
        } else {
            0
        };
    let (mut delays, mut pending_delay, mut loops) = (Vec::new(), 0, None);
    let mut scanned_duration = 0u32;
    loop {
        let kind = *data.get(cursor).ok_or(Error::InvalidImage)?;
        cursor += 1;
        match kind {
            0x3b => {
                if cursor != data.len() {
                    return Err(Error::InvalidImage);
                }
                break;
            }
            0x21 => {
                let label = *data.get(cursor).ok_or(Error::InvalidImage)?;
                cursor += 1;
                if label == 0xf9 {
                    let block = data.get(cursor..cursor + 6).ok_or(Error::InvalidImage)?;
                    if block[0] != 4 || block[5] != 0 {
                        return Err(Error::InvalidImage);
                    }
                    pending_delay = u32::from(u16::from_le_bytes([block[2], block[3]])) * 10;
                    cursor += 6;
                } else if label == 0xff {
                    let size = usize::from(*data.get(cursor).ok_or(Error::InvalidImage)?);
                    cursor += 1;
                    let app = data.get(cursor..cursor + size).ok_or(Error::InvalidImage)?;
                    cursor += size;
                    if app == b"NETSCAPE2.0" || app == b"ANIMEXTS1.0" {
                        if loops.is_some() {
                            return Err(Error::InvalidImage);
                        }
                        let block = data.get(cursor..cursor + 5).ok_or(Error::InvalidImage)?;
                        if block[0] != 3 || block[1] != 1 || block[4] != 0 {
                            return Err(Error::InvalidImage);
                        }
                        let n = u16::from_le_bytes([block[2], block[3]]);
                        loops = Some(if n == 0 {
                            0
                        } else {
                            n.checked_add(1).ok_or(Error::UnrepresentableAnimation)?
                        });
                        cursor += 5;
                    } else {
                        if app == b"ICCRGBG1012" {
                            return Err(Error::UnsupportedColorProfile);
                        }
                        skip_subblocks(data, &mut cursor)?
                    }
                } else {
                    skip_subblocks(data, &mut cursor)?
                }
            }
            0x2c => {
                let d = data.get(cursor..cursor + 9).ok_or(Error::InvalidImage)?;
                cursor += 9;
                let left = u32::from(u16::from_le_bytes([d[0], d[1]]));
                let top = u32::from(u16::from_le_bytes([d[2], d[3]]));
                let w = u32::from(u16::from_le_bytes([d[4], d[5]]));
                let h = u32::from(u16::from_le_bytes([d[6], d[7]]));
                if w == 0 || h == 0 || left + w > width || top + h > height {
                    return Err(Error::InvalidImage);
                }
                if d[8] & 128 != 0 {
                    cursor += 3 * (1usize << ((d[8] & 7) + 1))
                }
                cursor = cursor.checked_add(1).ok_or(Error::InvalidImage)?;
                skip_subblocks(data, &mut cursor)?;
                if delays.len() >= limits.frames as usize {
                    return Err(Error::ImageLimitsExceeded);
                }
                delays.push(pending_delay);
                scanned_duration = scanned_duration
                    .checked_add(pending_delay)
                    .ok_or(Error::ImageLimitsExceeded)?;
                if delays.len() > 1 && scanned_duration > limits.duration_ms {
                    return Err(Error::ImageLimitsExceeded);
                }
                pending_delay = 0;
            }
            _ => return Err(Error::InvalidImage),
        }
    }
    if delays.is_empty() {
        return Err(Error::InvalidImage);
    }
    let animated = delays.len() > 1;
    if animated && delays.contains(&0) {
        return Err(Error::UnrepresentableAnimation);
    }
    if !animated {
        delays[0] = 0
    }
    let duration_ms = delays
        .iter()
        .try_fold(0u32, |a, b| a.checked_add(*b))
        .ok_or(Error::ImageLimitsExceeded)?;
    Ok(GifStructure {
        facts: ImageFacts {
            width,
            height,
            frame_count: delays.len() as u32,
            duration_ms,
            loop_count: if animated { loops.unwrap_or(1) } else { 1 },
        },
        delays,
    })
}

pub fn validate_png(data: &[u8]) -> Result<bool> {
    if !data.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(Error::InvalidImage);
    }
    let mut cursor = 8;
    let mut end_seen = false;
    let mut header_seen = false;
    let mut srgb = false;
    let mut gamma = None;
    let mut chroma = false;
    let mut icc = false;
    while cursor < data.len() {
        let header = data.get(cursor..cursor + 8).ok_or(Error::InvalidImage)?;
        let length =
            u32::from_be_bytes(header[..4].try_into().map_err(|_| Error::InvalidImage)?) as usize;
        let end = cursor
            .checked_add(8)
            .and_then(|x| x.checked_add(length))
            .ok_or(Error::InvalidImage)?;
        let body = data.get(cursor + 8..end).ok_or(Error::InvalidImage)?;
        let crc = data.get(end..end + 4).ok_or(Error::InvalidImage)?;
        if crc32fast::hash(&data[cursor + 4..end])
            != u32::from_be_bytes(crc.try_into().map_err(|_| Error::InvalidImage)?)
        {
            return Err(Error::InvalidImage);
        }
        match &header[4..8] {
            b"acTL" => return Err(Error::UnsupportedFormat),
            b"IHDR" => {
                if cursor != 8 || length != 13 {
                    return Err(Error::InvalidImage);
                }
                header_seen = true;
                if !matches!(body[8], 1 | 2 | 4 | 8) {
                    return Err(Error::UnsupportedColorProfile);
                }
            }
            b"cICP" | b"mDCV" | b"cLLI" => return Err(Error::UnsupportedColorProfile),
            b"sRGB" => srgb = true,
            b"iCCP" => {
                if icc {
                    return Err(Error::UnsupportedColorProfile);
                }
                icc = true;
            }
            b"gAMA" => {
                if body.len() != 4 {
                    return Err(Error::InvalidImage);
                }
                gamma = Some(u32::from_be_bytes(
                    body.try_into().map_err(|_| Error::InvalidImage)?,
                ))
            }
            b"cHRM" => chroma = true,
            b"IEND" => {
                if length != 0 || end + 4 != data.len() {
                    return Err(Error::InvalidImage);
                }
                end_seen = true;
            }
            _ => {}
        }
        cursor = end + 4;
    }
    if !end_seen || !header_seen {
        return Err(Error::InvalidImage);
    }
    // Reject custom PNG transfer functions or chromaticity without a profile. Do not assume sRGB.
    if !icc && !srgb && (chroma || gamma.is_some_and(|g| g != 45455)) {
        return Err(Error::UnsupportedColorProfile);
    }
    Ok(icc)
}

pub fn validate_jpeg(data: &[u8]) -> Result<bool> {
    if !data.starts_with(&[255, 216]) {
        return Err(Error::InvalidImage);
    }
    let mut cursor = 2;
    let mut scan = false;
    let mut icc = false;
    while cursor < data.len() {
        if data[cursor] != 255 {
            if scan {
                cursor += 1;
                continue;
            } else {
                return Err(Error::InvalidImage);
            }
        }
        while data.get(cursor) == Some(&255) {
            cursor += 1
        }
        let marker = *data.get(cursor).ok_or(Error::InvalidImage)?;
        cursor += 1;
        if scan && (marker == 0 || matches!(marker, 0xd0..=0xd7)) {
            continue;
        }
        if marker == 0xd9 {
            return if cursor == data.len() {
                Ok(icc)
            } else {
                Err(Error::InvalidImage)
            };
        }
        if marker == 0xd8 || marker == 0x01 {
            return Err(Error::InvalidImage);
        }
        let length_data = data.get(cursor..cursor + 2).ok_or(Error::InvalidImage)?;
        let length = usize::from(u16::from_be_bytes([length_data[0], length_data[1]]));
        if length < 2 {
            return Err(Error::InvalidImage);
        }
        let segment = data
            .get(cursor + 2..cursor + length)
            .ok_or(Error::InvalidImage)?;
        if marker == 0xe2 && segment.starts_with(b"ICC_PROFILE\0") {
            icc = true;
        }
        if matches!(marker,0xc0..=0xc3|0xc5..=0xc7|0xc9..=0xcb|0xcd..=0xcf) {
            if segment.len() < 6 {
                return Err(Error::InvalidImage);
            }
            if segment[0] != 8 || !matches!(segment[5], 1 | 3) {
                return Err(Error::UnsupportedColorProfile);
            }
        }
        scan = marker == 0xda;
        cursor += length;
    }
    Err(Error::InvalidImage)
}
