use application::error::{ProcessingError as Error, Result};
use image::{DynamicImage, RgbaImage, metadata::Orientation};
pub(crate) fn profile(icc: &[u8]) -> Result<moxcms::ColorProfile> {
    if icc.len() < 132
        || icc.len() > 1 << 20
        || u32::from_be_bytes(
            icc[..4]
                .try_into()
                .map_err(|_| Error::UnsupportedColorProfile)?,
        ) as usize
            != icc.len()
    {
        return Err(Error::UnsupportedColorProfile);
    }
    let profile =
        moxcms::ColorProfile::new_from_slice(icc).map_err(|_| Error::UnsupportedColorProfile)?;
    if profile
        .cicp
        .is_some_and(|c| matches!(c.transfer_characteristics as u8, 16 | 18))
    {
        return Err(Error::UnsupportedColorProfile);
    }
    Ok(profile)
}

pub(crate) fn cicp_icc(primaries: u16, transfer: u16) -> Result<Vec<u8>> {
    cicp_profile(primaries, transfer)?
        .encode()
        .map_err(|_| Error::UnsupportedColorProfile)
}

pub(crate) fn cicp_profile(primaries: u16, transfer: u16) -> Result<moxcms::ColorProfile> {
    if matches!(transfer, 16 | 18) {
        return Err(Error::UnsupportedColorProfile);
    }
    let primaries = if matches!(primaries, 0 | 2) {
        1
    } else {
        primaries
    };
    let transfer = if matches!(transfer, 0 | 2) {
        13
    } else {
        transfer
    };
    let mut profile = moxcms::ColorProfile::new_srgb();
    let cicp = moxcms::CicpProfile {
        color_primaries: moxcms::CicpColorPrimaries::try_from(
            u8::try_from(primaries).map_err(|_| Error::UnsupportedColorProfile)?,
        )
        .map_err(|_| Error::UnsupportedColorProfile)?,
        transfer_characteristics: moxcms::TransferCharacteristics::try_from(
            u8::try_from(transfer).map_err(|_| Error::UnsupportedColorProfile)?,
        )
        .map_err(|_| Error::UnsupportedColorProfile)?,
        matrix_coefficients: moxcms::MatrixCoefficients::Identity,
        full_range: true,
    };
    let _: moxcms::ColorPrimaries = cicp
        .color_primaries
        .try_into()
        .map_err(|_| Error::UnsupportedColorProfile)?;
    let _: moxcms::ToneReprCurve = cicp
        .transfer_characteristics
        .try_into()
        .map_err(|_| Error::UnsupportedColorProfile)?;
    if primaries != 1 || transfer != 13 {
        // moxcms 0.9.1 returns false even after updating the profile. Validate the resulting tag.
        profile.update_rgb_colorimetry_from_cicp(cicp);
        if profile.cicp.is_none() {
            return Err(Error::UnsupportedColorProfile);
        }
    }
    Ok(profile)
}

pub(crate) fn normalize_dynamic(
    image: DynamicImage,
    orientation: Orientation,
    icc: Option<&[u8]>,
) -> Result<RgbaImage> {
    let profile = icc.map(profile).transpose()?;
    normalize_dynamic_with_profile(image, orientation, profile.as_ref())
}

pub(crate) fn normalize_dynamic_with_profile(
    image: DynamicImage,
    orientation: Orientation,
    profile: Option<&moxcms::ColorProfile>,
) -> Result<RgbaImage> {
    if image.color().bits_per_pixel() / u16::from(image.color().channel_count()) <= 8 {
        return normalize_with_profile(image.into_rgba8(), orientation, profile);
    }
    let mut pixels = image.into_rgba16();
    if let Some(profile) = profile {
        if profile.color_space != moxcms::DataColorSpace::Rgb {
            return Err(Error::UnsupportedColorProfile);
        }
        let transform = profile
            .create_transform_16bit(
                moxcms::Layout::Rgba,
                &moxcms::ColorProfile::new_srgb(),
                moxcms::Layout::Rgba,
                Default::default(),
            )
            .map_err(|_| Error::UnsupportedColorProfile)?;
        let mut output = vec![0; pixels.as_raw().len()];
        transform
            .transform(pixels.as_raw(), &mut output)
            .map_err(|_| Error::UnsupportedColorProfile)?;
        pixels = image::ImageBuffer::from_raw(pixels.width(), pixels.height(), output)
            .ok_or(Error::InternalError)?;
    }
    normalize_with_profile(
        DynamicImage::ImageRgba16(pixels).into_rgba8(),
        orientation,
        None,
    )
}
pub fn normalize(
    pixels: RgbaImage,
    orientation: Orientation,
    icc: Option<&[u8]>,
) -> Result<RgbaImage> {
    let profile = icc.map(profile).transpose()?;
    normalize_with_profile(pixels, orientation, profile.as_ref())
}

fn normalize_with_profile(
    mut pixels: RgbaImage,
    orientation: Orientation,
    profile: Option<&moxcms::ColorProfile>,
) -> Result<RgbaImage> {
    if let Some(profile) = profile {
        if profile.color_space != moxcms::DataColorSpace::Rgb {
            return Err(Error::UnsupportedColorProfile);
        }
        let transform = profile
            .create_transform_8bit(
                moxcms::Layout::Rgba,
                &moxcms::ColorProfile::new_srgb(),
                moxcms::Layout::Rgba,
                Default::default(),
            )
            .map_err(|_| Error::UnsupportedColorProfile)?;
        let mut output = vec![0; pixels.as_raw().len()];
        transform
            .transform(pixels.as_raw(), &mut output)
            .map_err(|_| Error::UnsupportedColorProfile)?;
        pixels = RgbaImage::from_raw(pixels.width(), pixels.height(), output)
            .ok_or(Error::InternalError)?;
    }
    let mut image = DynamicImage::ImageRgba8(pixels);
    image.apply_orientation(orientation);
    Ok(image.into_rgba8())
}
