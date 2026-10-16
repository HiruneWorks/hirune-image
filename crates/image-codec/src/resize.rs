use image::{Rgba, RgbaImage};
use std::sync::OnceLock;

pub fn resize_inside(source: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    if source.dimensions() == (width, height) {
        return source.clone();
    }
    static LINEAR: OnceLock<[f64; 256]> = OnceLock::new();
    let linear = LINEAR.get_or_init(|| {
        std::array::from_fn(|i| {
            let v = i as f64 / 255.;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        })
    });
    let sx = f64::from(source.width()) / f64::from(width);
    let sy = f64::from(source.height()) / f64::from(height);
    RgbaImage::from_fn(width, height, |x, y| {
        let (left, right, top, bottom) = (
            f64::from(x) * sx,
            f64::from(x + 1) * sx,
            f64::from(y) * sy,
            f64::from(y + 1) * sy,
        );
        let mut sum = [0.; 4];
        let area = (right - left) * (bottom - top);
        for iy in top.floor() as u32..(bottom.ceil() as u32).min(source.height()) {
            let wy = (bottom.min(f64::from(iy + 1)) - top.max(f64::from(iy))).max(0.);
            for ix in left.floor() as u32..(right.ceil() as u32).min(source.width()) {
                let weight = wy * (right.min(f64::from(ix + 1)) - left.max(f64::from(ix))).max(0.);
                let p = source.get_pixel(ix, iy).0;
                let a = f64::from(p[3]) / 255.;
                for c in 0..3 {
                    sum[c] += linear[usize::from(p[c])] * a * weight
                }
                sum[3] += a * weight;
            }
        }
        let mut out = [0; 4];
        out[3] = (sum[3] / area * 255.).round().clamp(0., 255.) as u8;
        if sum[3] > 0. {
            for c in 0..3 {
                let v = (sum[c] / sum[3]).clamp(0., 1.);
                let srgb = if v <= 0.0031308 {
                    v * 12.92
                } else {
                    1.055 * v.powf(1. / 2.4) - 0.055
                };
                out[c] = (srgb * 255.).round().clamp(0., 255.) as u8;
            }
        }
        Rgba(out)
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transparent_rgb_does_not_bleed_and_light_is_linear() {
        let image = RgbaImage::from_raw(2, 1, vec![255, 0, 0, 255, 0, 0, 255, 0]).unwrap();
        assert_eq!(
            resize_inside(&image, 1, 1).get_pixel(0, 0).0,
            [255, 0, 0, 128]
        );
        let image = RgbaImage::from_raw(2, 1, vec![0, 0, 0, 255, 255, 255, 255, 255]).unwrap();
        assert_eq!(
            resize_inside(&image, 1, 1).get_pixel(0, 0).0,
            [188, 188, 188, 255]
        );
    }
}
