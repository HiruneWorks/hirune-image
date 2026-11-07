use crate::normalize::{cicp_profile, profile};
use application::error::{ProcessingError as Error, Result};
use moxcms::{Chromaticity, ColorPrimaries, ColorProfile, Matrix3d, ToneReprCurve, XyY};

const SRGB_CHROMATICITIES: [u32; 8] = [31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000];

#[derive(Default)]
pub(crate) struct PngColor {
    pub has_icc: bool,
    pub srgb: Option<Result<u8>>,
    pub gamma: Option<Result<u32>>,
    pub chromaticities: Option<Result<[u32; 8]>>,
    pub cicp: Option<[u8; 4]>,
}

impl PngColor {
    pub fn profile(&self, embedded_icc: Option<&[u8]>) -> Result<Option<ColorProfile>> {
        if let Some([primaries, transfer, matrix, full_range]) = self.cicp {
            if matrix != 0
                || full_range != 1
                || matches!(primaries, 0 | 2)
                || matches!(transfer, 0 | 2)
            {
                return Err(Error::UnsupportedColorProfile);
            }
            return cicp_profile(u16::from(primaries), u16::from(transfer)).map(Some);
        }
        if self.has_icc {
            return profile(embedded_icc.ok_or(Error::UnsupportedColorProfile)?).map(Some);
        }
        if self.srgb.transpose()?.is_some() {
            return Ok(None);
        }
        let gamma = self.gamma.transpose()?;
        let chromaticities = self.chromaticities.transpose()?;
        // Keep the conventional sRGB interpretation of its standard fallback values.
        if gamma.is_none_or(|value| value == 45455)
            && chromaticities.is_none_or(|values| values == SRGB_CHROMATICITIES)
        {
            return Ok(None);
        }
        let mut profile = ColorProfile::new_srgb();
        profile.cicp = None;
        if let Some(values) = chromaticities {
            set_chromaticities(&mut profile, values)?;
        }
        if let Some(gamma) = gamma {
            let exponent = 100000.0 / gamma as f32;
            let curve = ToneReprCurve::Parametric(vec![exponent]);
            profile.red_trc = Some(curve.clone());
            profile.green_trc = Some(curve.clone());
            profile.blue_trc = Some(curve);
        }
        Ok(Some(profile))
    }
}

fn set_chromaticities(profile: &mut ColorProfile, values: [u32; 8]) -> Result<()> {
    for [x, y] in values.as_chunks::<2>().0 {
        if *y == 0 || u64::from(*x) + u64::from(*y) > 100000 {
            return Err(Error::UnsupportedColorProfile);
        }
    }
    let [rx, ry, gx, gy, bx, by] = values[2..]
        .try_into()
        .map(|values: [u32; 6]| values.map(i64::from))
        .map_err(|_| Error::UnsupportedColorProfile)?;
    if (gx - rx) * (by - ry) == (bx - rx) * (gy - ry) {
        return Err(Error::UnsupportedColorProfile);
    }
    let point = |offset: usize| Chromaticity {
        x: values[offset] as f32 / 100000.0,
        y: values[offset + 1] as f32 / 100000.0,
    };
    let primaries = ColorPrimaries {
        red: point(2),
        green: point(4),
        blue: point(6),
    };
    let xyz = [
        primaries.red.to_xyzd(),
        primaries.green.to_xyzd(),
        primaries.blue.to_xyzd(),
    ];
    let basis = Matrix3d {
        v: [
            [xyz[0].x, xyz[1].x, xyz[2].x],
            [xyz[0].y, xyz[1].y, xyz[2].y],
            [xyz[0].z, xyz[1].z, xyz[2].z],
        ],
    };
    // The CMS substitutes identity for a singular inverse. Reject that source description first.
    if basis
        .determinant()
        .is_none_or(|value| !value.is_finite() || value.abs() < 1e-12)
    {
        return Err(Error::UnsupportedColorProfile);
    }
    let white_point = XyY {
        x: f64::from(values[0]) / 100000.0,
        y: f64::from(values[1]) / 100000.0,
        yb: 1.0,
    };
    profile.update_rgb_colorimetry(white_point, primaries);
    profile.media_white_point = Some(white_point.to_xyzd());
    let matrix = profile.rgb_to_xyz_matrix();
    if matrix
        .v
        .iter()
        .flatten()
        .any(|value| !value.is_finite() || value.abs() > f64::from(f32::MAX))
        || matrix
            .determinant()
            .is_none_or(|value| !value.is_finite() || value.abs() < 1e-12)
    {
        return Err(Error::UnsupportedColorProfile);
    }
    Ok(())
}
