use application::images::{Encoding, Limits};
use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use image::{ImageFormat, Rgba, RgbaImage, metadata::Orientation};
use std::{hint::black_box, io::Cursor};

fn pipeline(c: &mut Criterion) {
    let pixels = RgbaImage::from_fn(1024, 768, |x, y| {
        Rgba([
            (x % 251) as u8,
            (y % 251) as u8,
            ((x ^ y) % 251) as u8,
            if x % 7 == 0 { 64 } else { 255 },
        ])
    });
    let mut png = Cursor::new(Vec::new());
    pixels.write_to(&mut png, ImageFormat::Png).unwrap();
    let limits = Limits::default();
    let mut group = c.benchmark_group("image_pipeline");
    group.throughput(Throughput::Elements(1024 * 768));
    // Prepare input before timing. The iter measurements include output allocation and drop.
    group.bench_function("decode_png_rgba", |b| {
        b.iter(|| {
            black_box(
                image_codec::decode_source(
                    black_box(png.get_ref()),
                    &limits,
                    || {},
                    |pixels, _| {
                        black_box(pixels);
                        Ok(())
                    },
                )
                .unwrap(),
            );
        })
    });
    group.bench_function("orient_rgba", |b| {
        b.iter_batched(
            || pixels.clone(),
            |pixels| {
                black_box(image_codec::normalize(pixels, Orientation::Rotate90, None).unwrap())
            },
            BatchSize::LargeInput,
        )
    });
    group.bench_function("resize_linear_area_256x192", |b| {
        b.iter(|| black_box(image_codec::resize_inside(black_box(&pixels), 256, 192)))
    });
    for (name, encoding) in [
        (
            "webp_lossy_q85_e4",
            Encoding::Lossy {
                quality: 85,
                effort: 4,
            },
        ),
        ("webp_lossless_e4", Encoding::Lossless { effort: 4 }),
    ] {
        group.bench_function(name, |b| {
            b.iter(|| {
                black_box(
                    image_codec::encode(
                        black_box(pixels.as_raw()),
                        1024,
                        768,
                        &encoding,
                        limits.output_bytes,
                    )
                    .unwrap(),
                )
            })
        });
    }
    group.finish();
}
criterion_group!(benches, pipeline);
criterion_main!(benches);
