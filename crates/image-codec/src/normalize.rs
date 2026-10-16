use application::error::{ProcessingError as Error, Result};
use image::{DynamicImage, RgbaImage, metadata::Orientation};
pub fn normalize(
    mut pixels: RgbaImage,
    orientation: Orientation,
    icc: Option<&[u8]>,
) -> Result<RgbaImage> {
    if let Some(icc) = icc {
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
        let profile = moxcms::ColorProfile::new_from_slice(icc)
            .map_err(|_| Error::UnsupportedColorProfile)?;
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
