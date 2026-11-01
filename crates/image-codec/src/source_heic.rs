use crate::{
    normalize::{cicp_icc, normalize_dynamic},
    source::{Description, Emit, static_facts},
};
use application::{
    error::{ProcessingError as Error, Result},
    images::{Limits, SourceFormat},
};
use image::{DynamicImage, metadata::Orientation};
use std::collections::HashMap;

pub(crate) struct ContainerInfo {
    pub avif: bool,
    pub tracks: bool,
}

pub(crate) fn inspect(bytes: &[u8]) -> Result<ContainerInfo> {
    let mut cursor = 0usize;
    let mut avif = false;
    let mut heic = false;
    let mut tracks = false;
    while cursor < bytes.len() {
        let header = bytes.get(cursor..cursor + 8).ok_or(Error::InvalidImage)?;
        let size = u32::from_be_bytes(header[..4].try_into().map_err(|_| Error::InvalidImage)?);
        let (size, offset) = if size == 1 {
            let value = bytes
                .get(cursor + 8..cursor + 16)
                .ok_or(Error::InvalidImage)?;
            (
                usize::try_from(u64::from_be_bytes(
                    value.try_into().map_err(|_| Error::InvalidImage)?,
                ))
                .map_err(|_| Error::InvalidImage)?,
                16,
            )
        } else if size == 0 {
            (bytes.len() - cursor, 8)
        } else {
            (size as usize, 8)
        };
        if size < offset {
            return Err(Error::InvalidImage);
        }
        let end = cursor.checked_add(size).ok_or(Error::InvalidImage)?;
        let body = bytes.get(cursor + offset..end).ok_or(Error::InvalidImage)?;
        match &header[4..8] {
            b"ftyp" => {
                if body.len() < 8 || body.len() % 4 != 0 {
                    return Err(Error::InvalidImage);
                }
                for brand in body[..4]
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .chain(body[8..].as_chunks::<4>().0.iter())
                {
                    avif |= matches!(brand, b"avif" | b"avis");
                    heic |= matches!(
                        brand,
                        b"heic" | b"heix" | b"hevc" | b"hevx" | b"mif1" | b"msf1"
                    );
                    tracks |= matches!(brand, b"avis" | b"hevc" | b"hevx" | b"msf1");
                }
            }
            b"moov" => tracks = true,
            _ => {}
        }
        cursor = end;
    }
    if !avif && !heic {
        return Err(Error::UnsupportedFormat);
    }
    Ok(ContainerInfo { avif, tracks })
}
pub(crate) fn is_avif(bytes: &[u8]) -> Result<bool> {
    Ok(inspect(bytes)?.avif)
}

pub(crate) fn decode(bytes: &[u8], limits: &Limits, emit: &mut Emit<'_>) -> Result<Description> {
    if inspect(bytes)?.tracks {
        return Err(Error::UnrepresentableAnimation);
    }
    let info = heic_rs::probe(bytes).map_err(|_| Error::InvalidImage)?;
    let mut facts = static_facts(info.width, info.height);
    limits.preflight(&facts)?;
    limits.canvas(info.coded_width, info.coded_height)?;
    let context = heic_rs::context::Context::open(bytes).map_err(|_| Error::InvalidImage)?;
    let props = context
        .props(context.meta.primary)
        .map_err(|_| Error::InvalidImage)?;
    // Inspect SPS geometry for tiles and alpha before the HEVC decoder allocates their planes.
    let mut coded = HashMap::new();
    for item in context.meta.items.iter().filter(|item| item.is_hevc()) {
        let item_props = context.props(item.id).map_err(|_| Error::InvalidImage)?;
        let hvcc = item_props.hvcc.ok_or(Error::InvalidImage)?;
        let mut largest = 0;
        for parameter in hvcc.parameter_sets() {
            if parameter.first().is_some_and(|b| (b >> 1) & 63 == 33) {
                let (width, height) = crate::heic_dimensions::coded_size(parameter)?;
                largest = largest.max(limits.canvas(width, height)?);
            }
        }
        if largest == 0 {
            return Err(Error::InvalidImage);
        }
        coded.insert(item.id, largest);
    }
    let mut coded_pixels = decoded_item_work(&context, context.meta.primary, &coded, limits)?;
    if let Some(alpha) = context
        .alpha_item(context.meta.primary)
        .map_err(|_| Error::InvalidImage)?
    {
        coded_pixels = coded_pixels
            .checked_add(decoded_item_work(&context, alpha, &coded, limits)?)
            .ok_or(Error::ImageLimitsExceeded)?;
    }
    check_coded_work(coded_pixels, limits)?;
    let nclx = props.nclx.unwrap_or_default();
    if matches!(nclx.transfer, 16 | 18)
        || matches!(nclx.matrix, heic_rs::MatrixCoefficients::Other(_))
    {
        return Err(Error::UnsupportedColorProfile);
    }
    let generated_icc;
    let icc = if let Some(profile) = props.icc {
        profile
    } else {
        generated_icc = cicp_icc(nclx.primaries, nclx.transfer)?;
        &generated_icc
    };
    let options = heic_rs::DecodeOptions::default()
        .with_layout(heic_rs::PixelLayout::Rgba16)
        .with_max_pixels(Some(limits.canvas_pixels.min(limits.working_bytes / 20)))
        .with_transforms(true)
        .with_strict(true)
        .with_threads(Some(1));
    let decoded = heic_rs::decode(bytes, &options).map_err(|error| match error {
        heic_rs::Error::PixelLimit { .. } => Error::ImageLimitsExceeded,
        heic_rs::Error::Unsupported(_) => Error::UnsupportedFormat,
        _ => Error::InvalidImage,
    })?;
    let samples: Vec<u16> = decoded
        .data
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_ne_bytes([b[0], b[1]]))
        .collect();
    let pixels = image::ImageBuffer::from_raw(decoded.width, decoded.height, samples)
        .ok_or(Error::InvalidImage)?;
    let orientation = if props.transforms.iter().any(|t| {
        matches!(
            t,
            heic_rs::Transform::Rotate(_) | heic_rs::Transform::Mirror(_)
        )
    }) {
        Orientation::NoTransforms
    } else {
        context
            .exif(context.meta.primary)
            .map_err(|_| Error::InvalidImage)?
            .and_then(Orientation::from_exif_chunk)
            .unwrap_or(Orientation::NoTransforms)
    };
    let pixels = normalize_dynamic(DynamicImage::ImageRgba16(pixels), orientation, Some(icc))?;
    facts.width = pixels.width();
    facts.height = pixels.height();
    emit(pixels, 0, Orientation::NoTransforms, None)?;
    Ok((SourceFormat::Heic, facts, 1))
}

fn check_coded_work(pixels: u64, limits: &Limits) -> Result<()> {
    if pixels > limits.total_frame_pixels
        || pixels
            .checked_mul(20)
            .is_none_or(|bytes| bytes > limits.working_bytes)
    {
        return Err(Error::ImageLimitsExceeded);
    }
    Ok(())
}

fn decoded_item_work(
    context: &heic_rs::context::Context<'_>,
    id: u32,
    coded: &HashMap<u32, u64>,
    limits: &Limits,
) -> Result<u64> {
    let occurrences = match context.grid(id).map_err(|_| Error::InvalidImage)? {
        Some((_, tiles)) => tiles,
        None => vec![id],
    };
    let mut pixels = 0u64;
    for tile in occurrences {
        pixels = pixels
            .checked_add(*coded.get(&tile).ok_or(Error::InvalidImage)?)
            .ok_or(Error::ImageLimitsExceeded)?;
        check_coded_work(pixels, limits)?;
    }
    Ok(pixels)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_grid_tile_references_count_each_decoded_plane() {
        use heic_rs::meta::{
            iloc::{Construction, Extent, ItemLocation},
            iref::{ItemReference, RefKind},
        };
        let bytes = include_bytes!("../../../tests/fixtures/green-alpha.heic");
        let mut context = heic_rs::context::Context::open(bytes).unwrap();
        let tile = context
            .meta
            .items
            .iter()
            .find(|item| item.is_hevc())
            .unwrap()
            .clone();
        let tile_id = tile.id;
        let grid_id = 60000;
        let mut grid = tile;
        grid.id = grid_id;
        grid.item_type = *b"grid";
        context.meta.items.push(grid);
        let grid_data = [0, 0, 0, 3, 0, 8, 0, 4];
        context.meta.idat = Some(&grid_data);
        context.meta.locations.push(ItemLocation {
            id: grid_id,
            construction: Construction::Idat,
            base_offset: 0,
            extents: vec![Extent {
                index: 0,
                offset: 0,
                length: 8,
            }],
        });
        context.meta.refs.push(ItemReference {
            kind: RefKind::Dimg,
            from: grid_id,
            to: vec![tile_id; 4],
        });
        let coded = HashMap::from([(tile_id, 4096)]);
        let limits = Limits {
            working_bytes: 3 * 4096 * 20,
            ..Limits::default()
        };
        assert_eq!(
            decoded_item_work(&context, grid_id, &coded, &limits),
            Err(Error::ImageLimitsExceeded)
        );
        let limits = Limits {
            working_bytes: 4 * 4096 * 20,
            ..limits
        };
        assert_eq!(
            decoded_item_work(&context, grid_id, &coded, &limits),
            Ok(4 * 4096)
        );
    }
}
