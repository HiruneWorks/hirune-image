use super::*;
use image::Rgba;

// Preserve the pre-optimization implementation as an independent pixel oracle.
fn reference(source: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    if source.dimensions() == (width, height) {
        return source.clone();
    }
    let values: [f64; 256] = std::array::from_fn(|i| {
        let v = i as f64 / 255.;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
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
                    sum[c] += values[usize::from(p[c])] * a * weight;
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

#[test]
fn transparent_rgb_does_not_bleed_and_light_is_linear() {
    let pixels = RgbaImage::from_raw(2, 1, vec![255, 0, 0, 255, 0, 0, 255, 0]).unwrap();
    assert_eq!(
        resize_inside(&pixels, 1, 1).get_pixel(0, 0).0,
        [255, 0, 0, 128]
    );
    let pixels = RgbaImage::from_raw(2, 1, vec![0, 0, 0, 255, 255, 255, 255, 255]).unwrap();
    assert_eq!(
        resize_inside(&pixels, 1, 1).get_pixel(0, 0).0,
        [188, 188, 188, 255]
    );
}

#[test]
fn cached_and_uncached_area_match_legacy_pixels_at_odd_sizes_and_alpha_boundaries() {
    let mut state = 0x2c6d_1378_191e_3a49u64;
    let mut next = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (state >> 32) as u32
    };
    for case in 0..400 {
        let (width, height) = (next() % 73 + 1, next() % 61 + 1);
        let output = (next() % width + 1, next() % height + 1);
        let pixels = RgbaImage::from_fn(width, height, |_, _| {
            let values = next().to_le_bytes();
            let alpha = match case % 7 {
                0 => 0,
                1 => 1,
                2 => 127,
                3 => 128,
                4 => 255,
                _ => values[3],
            };
            Rgba([values[0], values[1], values[2], alpha])
        });
        let expected = reference(&pixels, output.0, output.1);
        assert_eq!(
            resize_inside(&pixels, output.0, output.1),
            expected,
            "case {case}"
        );
        let mut plan = ResizePlan::new(pixels.dimensions(), output);
        plan.cache().unwrap();
        assert_eq!(plan.apply(&pixels), expected, "cached case {case}");
        assert_eq!(resize_inside(&pixels, 1, 1), reference(&pixels, 1, 1));
    }
}

#[test]
fn transfer_table_matches_legacy_inverse_including_half_code_neighbors() {
    let table = tables();
    for index in 0..1_000_000 {
        let value = (index as f64 + 0.37) / 1_000_000.;
        assert_eq!(table.encode(value), legacy_encode(value), "linear {value}");
    }
    for boundary in table.boundaries {
        for delta in [-1e-7, -1e-9, -1e-11, 0., 1e-11, 1e-9, 1e-7] {
            let value = boundary + delta;
            assert_eq!(table.encode(value), legacy_encode(value));
        }
        for bits in boundary.to_bits() - 2..=boundary.to_bits() + 2 {
            let value = f64::from_bits(bits);
            assert_eq!(table.encode(value), legacy_encode(value));
        }
    }
}

#[test]
fn geometry_cache_reserves_all_outputs_and_falls_back_without_changing_pixels() {
    let source = RgbaImage::from_fn(53, 29, |x, y| {
        Rgba([(x * 4) as u8, (y * 8) as u8, 97, ((x + y) % 256) as u8])
    });
    let outputs = [(17, 9), (7, 3), (53, 29)];
    let reservation = 53 * 29 * 20;
    let cache = outputs
        .iter()
        .map(|&output| ResizePlan::cache_bytes(output).unwrap())
        .sum::<u64>();
    assert!(
        cached_plans(
            source.dimensions(),
            &outputs,
            reservation,
            reservation + cache - 1
        )
        .unwrap()
        .is_none()
    );
    let plans = cached_plans(
        source.dimensions(),
        &outputs,
        reservation,
        reservation + cache,
    )
    .unwrap()
    .unwrap();
    for (plan, &(width, height)) in plans.iter().zip(&outputs) {
        assert_eq!(plan.apply(&source), reference(&source, width, height));
    }
}
