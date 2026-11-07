use crate::png_color::PngColor;
use application::error::{ProcessingError as Error, Result};

pub(crate) fn inspect(data: &[u8]) -> Result<PngColor> {
    if !data.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(Error::InvalidImage);
    }
    let mut cursor = 8usize;
    let mut header_seen = false;
    let mut end_seen = false;
    let mut before_palette_or_pixels = true;
    let mut color = PngColor::default();
    while cursor < data.len() {
        let header = data.get(cursor..cursor + 8).ok_or(Error::InvalidImage)?;
        let length =
            u32::from_be_bytes(header[..4].try_into().map_err(|_| Error::InvalidImage)?) as usize;
        let end = cursor
            .checked_add(8)
            .and_then(|value| value.checked_add(length))
            .ok_or(Error::InvalidImage)?;
        let next = end.checked_add(4).ok_or(Error::InvalidImage)?;
        let body = data.get(cursor + 8..end).ok_or(Error::InvalidImage)?;
        let crc = data.get(end..next).ok_or(Error::InvalidImage)?;
        let kind = &header[4..8];
        let discarded_metadata = matches!(
            kind,
            b"tEXt" | b"zTXt" | b"iTXt" | b"tIME" | b"pHYs" | b"bKGD" | b"hIST" | b"sPLT"
        );
        if !discarded_metadata
            && crc32fast::hash(&data[cursor + 4..end])
                != u32::from_be_bytes(crc.try_into().map_err(|_| Error::InvalidImage)?)
        {
            return Err(Error::InvalidImage);
        }
        match kind {
            b"IHDR" => {
                if cursor != 8 || length != 13 {
                    return Err(Error::InvalidImage);
                }
                header_seen = true;
                if !matches!(body[8], 1 | 2 | 4 | 8 | 16) {
                    return Err(Error::UnsupportedColorProfile);
                }
            }
            b"PLTE" | b"IDAT" => before_palette_or_pixels = false,
            b"mDCV" | b"cLLI" => return Err(Error::UnsupportedColorProfile),
            b"cICP" if before_palette_or_pixels && color.cicp.is_none() => {
                color.cicp = Some(body.try_into().map_err(|_| Error::InvalidImage)?);
            }
            b"sRGB" if before_palette_or_pixels && !matches!(color.srgb, Some(Ok(_))) => {
                color.srgb = Some(match body {
                    [intent] if *intent <= 3 => Ok(*intent),
                    _ => Err(Error::InvalidImage),
                });
            }
            b"iCCP" if before_palette_or_pixels => {
                if color.has_icc {
                    return Err(Error::UnsupportedColorProfile);
                }
                color.has_icc = true;
            }
            b"gAMA" if before_palette_or_pixels && !matches!(color.gamma, Some(Ok(_))) => {
                let value = body
                    .try_into()
                    .map(u32::from_be_bytes)
                    .map_err(|_| Error::InvalidImage);
                if value != Ok(0) {
                    color.gamma = Some(value);
                }
            }
            b"cHRM" if before_palette_or_pixels && !matches!(color.chromaticities, Some(Ok(_))) => {
                let bytes: Result<&[u8; 32]> = body.try_into().map_err(|_| Error::InvalidImage);
                color.chromaticities = Some(bytes.map(|bytes| {
                    std::array::from_fn(|index| {
                        u32::from_be_bytes([
                            bytes[index * 4],
                            bytes[index * 4 + 1],
                            bytes[index * 4 + 2],
                            bytes[index * 4 + 3],
                        ])
                    })
                }));
            }
            b"IEND" => {
                if length != 0 || next != data.len() {
                    return Err(Error::InvalidImage);
                }
                end_seen = true;
            }
            _ => {}
        }
        cursor = next;
    }
    if !header_seen || !end_seen {
        return Err(Error::InvalidImage);
    }
    Ok(color)
}
