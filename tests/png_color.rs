mod support;

use application::images::Limits;
use support::{Server, fixture, recipe, verify};

const SRGB_CHROMATICITIES: [u32; 8] = [31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000];

fn chunk(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    output.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    output.extend_from_slice(kind);
    output.extend_from_slice(payload);
    output.extend_from_slice(&crc32fast::hash(&output[4..]).to_be_bytes());
    output
}

fn chromaticities(values: [u32; 8]) -> Vec<u8> {
    chunk(
        b"cHRM",
        &values
            .into_iter()
            .flat_map(u32::to_be_bytes)
            .collect::<Vec<_>>(),
    )
}

fn replace_color_chunks(source: &[u8], chunks: &[Vec<u8>]) -> Vec<u8> {
    let mut output = source[..33].to_vec();
    output.extend(chunks.iter().flatten());
    let mut cursor = 33;
    while cursor < source.len() {
        let length = u32::from_be_bytes(source[cursor..cursor + 4].try_into().unwrap()) as usize;
        let next = cursor + length + 12;
        if !matches!(
            &source[cursor + 4..cursor + 8],
            b"iCCP" | b"sRGB" | b"cICP" | b"gAMA" | b"cHRM"
        ) {
            output.extend_from_slice(&source[cursor..next]);
        }
        cursor = next;
    }
    output
}

fn assert_pixels(output: &[u8], expected: &[u8], frame_count: usize, tolerance: u8) {
    let mut count = 0;
    image_codec::decode_frames(output, &Limits::default(), |pixels, _| {
        assert_eq!(pixels.len(), expected.len());
        for (actual, expected) in pixels
            .as_chunks::<4>()
            .0
            .iter()
            .zip(expected.as_chunks::<4>().0)
        {
            assert_eq!(actual[3], expected[3]);
            for channel in 0..3 {
                assert!(
                    actual[channel].abs_diff(expected[channel]) <= tolerance,
                    "{actual:?} != {expected:?}"
                );
            }
        }
        count += 1;
        Ok(())
    })
    .unwrap();
    assert_eq!(count, frame_count);
}

fn srgb_sample(linear: f64) -> u8 {
    let encoded = if linear <= 0.0031308 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round().clamp(0.0, 255.0) as u8
}

fn png_source(
    depth: png::BitDepth,
    animated: bool,
    frames: &[&[u8]],
    chunks: &[Vec<u8>],
) -> Vec<u8> {
    let mut source = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut source, 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(depth);
        if animated {
            encoder.set_animated(frames.len() as u32, 3).unwrap();
            encoder.set_frame_delay(1, 10).unwrap();
        }
        let mut writer = encoder.write_header().unwrap();
        for frame in frames {
            writer.write_image_data(frame).unwrap();
        }
        writer.finish().unwrap();
    }
    source.splice(33..33, chunks.iter().flatten().copied());
    source
}

#[tokio::test]
async fn standard_png_color_chunks_work_without_an_srgb_or_icc_chunk() {
    let server = Server::start(Limits::default()).await;
    let recipe = recipe("lossless", "preserve", &[(1, 1)]);
    for (depth, pixel) in [
        (png::BitDepth::Eight, vec![64, 128, 192, 128]),
        (
            png::BitDepth::Sixteen,
            vec![64, 64, 128, 128, 192, 192, 128, 128],
        ),
    ] {
        for animated in [false, true] {
            let frames = if animated {
                vec![pixel.as_slice(); 2]
            } else {
                vec![pixel.as_slice()]
            };
            for chunks in [
                vec![],
                vec![chunk(b"gAMA", &45455u32.to_be_bytes())],
                vec![chromaticities(SRGB_CHROMATICITIES)],
                vec![
                    chunk(b"gAMA", &45455u32.to_be_bytes()),
                    chromaticities(SRGB_CHROMATICITIES),
                ],
                vec![
                    chunk(b"gAMA", &0u32.to_be_bytes()),
                    chromaticities(SRGB_CHROMATICITIES),
                ],
            ] {
                let source = png_source(depth, animated, &frames, &chunks);
                let (manifest, outputs) = verify(
                    server.process(source.clone(), &recipe).await,
                    &source,
                    &recipe,
                )
                .await;
                assert_eq!(manifest.source.image.animated, animated);
                assert_eq!(manifest.source.image.frame_count, frames.len() as u32);
                assert_eq!(
                    manifest.source.image.loop_count,
                    if animated { 3 } else { 1 }
                );
                let mut count = 0;
                image_codec::decode_frames(&outputs[0], &Limits::default(), |pixels, delay| {
                    assert_eq!(pixels, [64, 128, 192, 128]);
                    assert_eq!(delay, if animated { 100 } else { 0 });
                    count += 1;
                    Ok(())
                })
                .unwrap();
                assert_eq!(count, frames.len());
            }
        }
    }
    server.released().await;
}

#[tokio::test]
async fn custom_gamma_converts_before_quantization_and_preserves_apng_alpha() {
    let server = Server::start(Limits::default()).await;
    let recipe = recipe("lossless", "preserve", &[(1, 1)]);
    for (depth, samples, maximum) in [
        (png::BitDepth::Eight, [64u16, 128, 192, 128], 255.0),
        (png::BitDepth::Sixteen, [1u16, 4660, 32896, 32896], 65535.0),
    ] {
        let pixel = if depth == png::BitDepth::Eight {
            samples
                .into_iter()
                .map(|value| value as u8)
                .collect::<Vec<_>>()
        } else {
            samples.into_iter().flat_map(u16::to_be_bytes).collect()
        };
        for gamma in [100000u32, 55000, 200000, 1, 3, u32::MAX] {
            let mut expected = samples[..3]
                .iter()
                .map(|value| {
                    srgb_sample((f64::from(*value) / maximum).powf(100000.0 / f64::from(gamma)))
                })
                .collect::<Vec<_>>();
            expected.push(128);
            for animated in [false, true] {
                let frames = vec![pixel.as_slice(); if animated { 2 } else { 1 }];
                let source = png_source(
                    depth,
                    animated,
                    &frames,
                    &[chunk(b"gAMA", &gamma.to_be_bytes())],
                );
                let (_, outputs) = verify(
                    server.process(source.clone(), &recipe).await,
                    &source,
                    &recipe,
                )
                .await;
                assert_pixels(&outputs[0], &expected, frames.len(), 1);
            }
        }
    }
    server.released().await;
}

#[tokio::test]
async fn png_primaries_and_cicp_match_independent_littlecms_pixels() {
    let server = Server::start(Limits::default()).await;
    let recipe = recipe("lossless", "preserve", &[(8, 4)]);
    let original = fixture("p3-color.png");
    let expected = fixture("p3-color-expected.rgba");
    let p3 = [31270, 32900, 68000, 32000, 26500, 69000, 15000, 6000];
    for chunks in [
        vec![chromaticities(p3)],
        vec![chunk(b"cICP", &[12, 13, 0, 1])],
        vec![
            chunk(b"cICP", &[12, 13, 0, 1]),
            chunk(b"sRGB", &[0]),
            chunk(b"gAMA", &100000u32.to_be_bytes()),
        ],
    ] {
        let source = replace_color_chunks(&original, &chunks);
        let (_, outputs) = verify(
            server.process(source.clone(), &recipe).await,
            &source,
            &recipe,
        )
        .await;
        assert_pixels(&outputs[0], &expected, 1, 2);
    }
    let raw = image::load_from_memory_with_format(&original, image::ImageFormat::Png)
        .unwrap()
        .into_rgba8()
        .into_raw();
    let mut source = original.clone();
    source.splice(33..33, chunk(b"cICP", &[1, 13, 0, 1]));
    let (_, outputs) = verify(
        server.process(source.clone(), &recipe).await,
        &source,
        &recipe,
    )
    .await;
    assert_pixels(&outputs[0], &raw, 1, 1);
    let mut source = original;
    source.splice(33..33, chunk(b"sRGB", &[0]));
    let (_, outputs) = verify(
        server.process(source.clone(), &recipe).await,
        &source,
        &recipe,
    )
    .await;
    assert_pixels(&outputs[0], &expected, 1, 2);
    server.released().await;
}

#[tokio::test]
async fn discarded_png_metadata_crc_errors_do_not_lose_pixels_or_frames() {
    let server = Server::start(Limits::default()).await;
    let recipe = recipe("lossless", "preserve", &[(1, 1)]);
    for animated in [false, true] {
        for kind in [
            b"tEXt", b"zTXt", b"iTXt", b"tIME", b"pHYs", b"bKGD", b"hIST", b"sPLT",
        ] {
            let mut metadata = chunk(kind, b"discarded synthetic metadata");
            *metadata.last_mut().unwrap() ^= 1;
            let frames: Vec<&[u8]> = vec![&[64, 128, 192, 128]; if animated { 2 } else { 1 }];
            let source = png_source(png::BitDepth::Eight, animated, &frames, &[metadata]);
            let (_, outputs) = verify(
                server.process(source.clone(), &recipe).await,
                &source,
                &recipe,
            )
            .await;
            assert_pixels(&outputs[0], &[64, 128, 192, 128], frames.len(), 0);
        }
    }
    server.released().await;
}

#[tokio::test]
async fn color_alpha_animation_and_pixel_corruption_remain_errors() {
    let server = Server::start(Limits::default()).await;
    let recipe = recipe("lossless", "preserve", &[(1, 1)]);
    for kind in [
        b"gAMA", b"cHRM", b"iCCP", b"sRGB", b"cICP", b"eXIf", b"tRNS", b"acTL", b"fcTL", b"fdAT",
    ] {
        let mut metadata = chunk(kind, &[0; 32]);
        *metadata.last_mut().unwrap() ^= 1;
        let source = png_source(
            png::BitDepth::Eight,
            false,
            &[&[64, 128, 192, 128]],
            &[metadata],
        );
        let response = server.process(source, &recipe).await;
        assert_eq!(response.status(), 422);
        assert_eq!(
            response.json::<serde_json::Value>().await.unwrap()["error"]["code"],
            "invalid_image"
        );
    }
    for values in [
        [9, 16, 0, 1],
        [9, 18, 0, 1],
        [1, 13, 0, 0],
        [1, 13, 1, 1],
        [255, 13, 0, 1],
        [1, 255, 0, 1],
    ] {
        let source = png_source(
            png::BitDepth::Eight,
            false,
            &[&[64, 128, 192, 128]],
            &[chunk(b"cICP", &values)],
        );
        let response = server.process(source, &recipe).await;
        assert_eq!(response.status(), 422);
        assert_eq!(
            response.json::<serde_json::Value>().await.unwrap()["error"]["code"],
            if values[2] == 0 {
                "unsupported_color_profile"
            } else {
                "invalid_image"
            }
        );
    }
    for values in [
        [0; 8],
        [31270, 32900, 10000, 40000, 20000, 30000, 30000, 20000],
        [31270, 32900, 64000, 33000, 64000, 33000, 64000, 33000],
        [31270, 32900, 68000, 33000, 30000, 60000, 15000, 6000],
    ] {
        let source = png_source(
            png::BitDepth::Eight,
            false,
            &[&[64, 128, 192, 128]],
            &[chromaticities(values)],
        );
        let response = server.process(source, &recipe).await;
        assert_eq!(response.status(), 422);
        assert_eq!(
            response.json::<serde_json::Value>().await.unwrap()["error"]["code"],
            "unsupported_color_profile"
        );
    }
    server.released().await;
}

#[tokio::test]
async fn png_non_d65_white_point_matches_independent_littlecms_pixels() {
    let server = Server::start(Limits::default()).await;
    let recipe = recipe("lossless", "preserve", &[(4, 1)]);
    let source = fixture("d50-calibration.png");
    let (_, outputs) = verify(
        server.process(source.clone(), &recipe).await,
        &source,
        &recipe,
    )
    .await;
    assert_pixels(&outputs[0], &fixture("d50-calibration-expected.rgba"), 1, 2);
    server.released().await;
}

#[tokio::test]
async fn selected_color_declarations_supersede_broken_or_out_of_order_fallbacks() {
    let server = Server::start(Limits::default()).await;
    let recipe = recipe("lossless", "preserve", &[(8, 4)]);
    let original = fixture("p3-color.png");
    let raw = image::load_from_memory_with_format(&original, image::ImageFormat::Png)
        .unwrap()
        .into_rgba8()
        .into_raw();
    let mut source = original.clone();
    source.splice(33..33, chunk(b"sRGB", &[255]));
    let (_, outputs) = verify(
        server.process(source.clone(), &recipe).await,
        &source,
        &recipe,
    )
    .await;
    assert_pixels(&outputs[0], &fixture("p3-color-expected.rgba"), 1, 2);
    let mut source = original;
    source.splice(33..33, chunk(b"PLTE", &[0, 0, 0]));
    let (_, outputs) = verify(
        server.process(source.clone(), &recipe).await,
        &source,
        &recipe,
    )
    .await;
    assert_pixels(&outputs[0], &raw, 1, 0);
    for icc in [chunk(b"iCCP", b"profile\0\0not a zlib stream"), {
        let mut encoded = Vec::new();
        let mut info = png::Info::with_size(1, 1);
        info.color_type = png::ColorType::Rgba;
        info.icc_profile = Some(std::borrow::Cow::Owned(vec![0; 132]));
        let mut writer = png::Encoder::with_info(&mut encoded, info)
            .unwrap()
            .write_header()
            .unwrap();
        writer.write_image_data(&[64, 128, 192, 128]).unwrap();
        writer.finish().unwrap();
        let mut cursor = 33;
        loop {
            let length =
                u32::from_be_bytes(encoded[cursor..cursor + 4].try_into().unwrap()) as usize;
            if &encoded[cursor + 4..cursor + 8] == b"iCCP" {
                break encoded[cursor..cursor + length + 12].to_vec();
            }
            cursor += length + 12;
        }
    }] {
        let failed = png_source(
            png::BitDepth::Eight,
            false,
            &[&[64, 128, 192, 128]],
            std::slice::from_ref(&icc),
        );
        assert_eq!(server.process(failed, &recipe).await.status(), 422);
        let selected = png_source(
            png::BitDepth::Eight,
            false,
            &[&[64, 128, 192, 128]],
            &[chunk(b"cICP", &[1, 13, 0, 1]), icc, chunk(b"sRGB", &[255])],
        );
        let (_, outputs) = verify(
            server.process(selected.clone(), &recipe).await,
            &selected,
            &recipe,
        )
        .await;
        assert_pixels(&outputs[0], &[64, 128, 192, 128], 1, 1);
    }
    server.released().await;
}

#[tokio::test]
async fn calibrated_apng_over_keeps_the_existing_encoded_sample_composition() {
    let mut source = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut source, 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_animated(2, 3).unwrap();
        encoder.set_frame_delay(1, 10).unwrap();
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[0, 0, 0, 255]).unwrap();
        writer.set_blend_op(png::BlendOp::Over).unwrap();
        writer.write_image_data(&[255, 255, 255, 128]).unwrap();
        writer.finish().unwrap();
    }
    source.splice(33..33, chunk(b"gAMA", &50000u32.to_be_bytes()));
    let server = Server::start(Limits::default()).await;
    let recipe = recipe("lossless", "preserve", &[(1, 1)]);
    let (manifest, outputs) = verify(
        server.process(source.clone(), &recipe).await,
        &source,
        &recipe,
    )
    .await;
    assert_eq!(manifest.outputs[0].image.frame_count, 2);
    assert_eq!(manifest.outputs[0].image.loop_count, 3);
    let mut frames = Vec::new();
    image_codec::decode_frames(&outputs[0], &Limits::default(), |pixels, delay| {
        assert_eq!(delay, 100);
        frames.push(pixels);
        Ok(())
    })
    .unwrap();
    let expected = srgb_sample((128.0f64 / 255.0).powi(2));
    assert_eq!(
        frames,
        [vec![0, 0, 0, 255], vec![expected, expected, expected, 255]]
    );
    server.released().await;
}

#[tokio::test]
async fn png_color_precedence_ignores_late_and_duplicate_fallback_values() {
    let server = Server::start(Limits::default()).await;
    let recipe = recipe("lossless", "preserve", &[(1, 1)]);
    let pixel: &[u8] = &[64, 128, 192, 128];
    for chunks in [
        vec![
            chunk(b"sRGB", &[0]),
            chunk(b"gAMA", &[0]),
            chunk(b"cHRM", &[0; 4]),
        ],
        vec![
            chunk(b"sRGB", &[0]),
            chunk(b"gAMA", &100000u32.to_be_bytes()),
            chromaticities([0; 8]),
        ],
        vec![
            chunk(b"gAMA", &45455u32.to_be_bytes()),
            chunk(b"gAMA", &100000u32.to_be_bytes()),
        ],
        vec![chromaticities(SRGB_CHROMATICITIES), chromaticities([0; 8])],
    ] {
        let source = png_source(png::BitDepth::Eight, false, &[pixel], &chunks);
        let (_, outputs) = verify(
            server.process(source.clone(), &recipe).await,
            &source,
            &recipe,
        )
        .await;
        assert_pixels(&outputs[0], pixel, 1, 0);
    }
    let mut source = png_source(png::BitDepth::Eight, false, &[pixel], &[]);
    source.splice(
        source.len() - 12..source.len() - 12,
        chunk(b"gAMA", &100000u32.to_be_bytes()),
    );
    let (_, outputs) = verify(
        server.process(source.clone(), &recipe).await,
        &source,
        &recipe,
    )
    .await;
    assert_pixels(&outputs[0], pixel, 1, 0);
    server.released().await;
}
