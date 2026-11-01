mod support;
#[tokio::test]
async fn timed_single_frame_gif_remains_an_animation() {
    let mut source = Vec::new();
    {
        let mut encoder = gif::Encoder::new(&mut source, 1, 1, &[255, 0, 0]).unwrap();
        encoder.set_repeat(gif::Repeat::Finite(2)).unwrap();
        encoder
            .write_frame(&gif::Frame {
                width: 1,
                height: 1,
                delay: 10,
                buffer: std::borrow::Cow::Borrowed(&[0]),
                ..Default::default()
            })
            .unwrap();
    }
    let server = Server::start(Limits::default()).await;
    let recipe = recipe("lossless", "preserve", &[(1, 1), (1, 1)]);
    let (manifest, _) = verify(
        server.process(source.clone(), &recipe).await,
        &source,
        &recipe,
    )
    .await;
    for output in manifest.outputs {
        assert!(output.image.animated);
        assert_eq!(output.image.frame_count, 1);
        assert_eq!(output.image.duration_ms, 100);
        assert_eq!(output.image.loop_count, 3);
    }
    server.released().await;
}

#[tokio::test]
async fn sixteen_bit_apng_keeps_alpha_and_all_display_frames() {
    let mut source = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut source, 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Sixteen);
        encoder.set_animated(2, 0).unwrap();
        encoder.set_frame_delay(1, 100).unwrap();
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_image_data(&[255, 255, 0, 0, 0, 0, 128, 128])
            .unwrap();
        writer
            .write_image_data(&[0, 0, 0, 0, 255, 255, 128, 128])
            .unwrap();
        writer.finish().unwrap();
    }
    let server = Server::start(Limits::default()).await;
    let recipe = recipe("lossless", "preserve", &[(1, 1)]);
    let (manifest, outputs) = verify(
        server.process(source.clone(), &recipe).await,
        &source,
        &recipe,
    )
    .await;
    assert!(manifest.outputs[0].image.animated);
    assert_eq!(manifest.outputs[0].image.frame_count, 2);
    let mut frames = Vec::new();
    image_codec::decode_frames(&outputs[0], &Limits::default(), |pixels, delay| {
        assert_eq!(delay, 10);
        frames.push(pixels);
        Ok(())
    })
    .unwrap();
    assert_eq!(frames, [vec![255, 0, 0, 128], vec![0, 0, 255, 128]]);
    server.released().await;
}
#[tokio::test]
async fn svg_embedded_p3_pixels_use_the_same_color_normalization_as_the_source() {
    let server = Server::start(Limits::default()).await;
    let source = fixture("p3-embedded.svg");
    let recipe = recipe("lossless", "preserve", &[(4096, 4096), (4, 2)]);
    let (_, outputs) = verify(
        server.process(source.clone(), &recipe).await,
        &source,
        &recipe,
    )
    .await;
    let expected = fixture("p3-color-expected.rgba");
    image_codec::decode_frames(&outputs[0], &Limits::default(), |pixels, _| {
        assert_eq!(pixels.len(), expected.len());
        for (actual, expected) in pixels
            .as_chunks::<4>()
            .0
            .iter()
            .zip(expected.as_chunks::<4>().0.iter())
        {
            assert_eq!(actual[3], expected[3]);
            for channel in 0..3 {
                let a = f64::from(actual[channel]) * f64::from(actual[3]) / 255.0;
                let b = f64::from(expected[channel]) * f64::from(expected[3]) / 255.0;
                assert!((a - b).abs() <= 2.0, "{actual:?} != {expected:?}");
            }
        }
        Ok(())
    })
    .unwrap();
    server.released().await;
}

#[tokio::test]
async fn svg_css_escapes_external_paints_and_stylesheet_processing_instructions_fail_explicitly() {
    let server = Server::start(Limits::default()).await;
    let recipe = recipe("lossless", "preserve", &[(8, 4)]);
    for content in [
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="4"><rect style="anim\61tion:spin 1s infinite"/></svg>"#,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="4"><rect fill="url(https://invalid.example/image)"/></svg>"#,
        r#"<?xml-stylesheet type="text/css" href="file:///etc/passwd"?><svg xmlns="http://www.w3.org/2000/svg" width="8" height="4"/>"#,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="4"><use href="data:image/svg+xml,%3Csvg/%3E"/></svg>"#,
    ] {
        assert_eq!(
            server
                .process(content.as_bytes().to_vec(), &recipe)
                .await
                .status(),
            415
        );
        server.released().await;
    }
}

#[tokio::test]
async fn heic_coded_geometry_is_checked_before_crop_and_timed_tracks_are_never_flattened() {
    let source = fixture("green-alpha.heic");
    let server = Server::start(Limits {
        canvas_pixels: 32,
        ..Limits::default()
    })
    .await;
    let response = server
        .process(source.clone(), &recipe("lossless", "preserve", &[(8, 4)]))
        .await;
    assert_eq!(response.status(), 413);
    assert_eq!(
        response.json::<serde_json::Value>().await.unwrap()["error"]["code"],
        "image_limits_exceeded"
    );
    server.released().await;
    let server = Server::start(Limits::default()).await;
    let mut track = source.clone();
    track.extend_from_slice(b"\0\0\0\x08moov");
    let mut compatible = source;
    compatible[16..20].copy_from_slice(b"msf1");
    for source in [track, compatible] {
        let response = server
            .process(source, &recipe("lossless", "preserve", &[(8, 4)]))
            .await;
        assert_eq!(response.status(), 422);
        assert_eq!(
            response.json::<serde_json::Value>().await.unwrap()["error"]["code"],
            "unrepresentable_animation"
        );
        server.released().await;
    }
}

#[tokio::test]
async fn multi_page_tiff_is_rejected_without_discarding_unrequested_pages() {
    let mut source = Cursor::new(Vec::new());
    {
        let mut encoder = tiff::encoder::TiffEncoder::new(&mut source).unwrap();
        encoder
            .write_image::<tiff::encoder::colortype::RGB8>(1, 1, &[255, 0, 0])
            .unwrap();
        encoder
            .write_image::<tiff::encoder::colortype::RGB8>(1, 1, &[0, 0, 255])
            .unwrap();
    }
    let server = Server::start(Limits::default()).await;
    assert_eq!(
        server
            .process(
                source.into_inner(),
                &recipe("lossless", "preserve", &[(1, 1)])
            )
            .await
            .status(),
        415
    );
    server.released().await;
}

#[tokio::test]
async fn combined_output_budget_is_enforced_across_all_animation_muxes() {
    let source = apng(false, 3, 0);
    let recipe = recipe("lossless", "preserve", &[(2, 1), (1, 1)]);
    let baseline = Server::start(Limits::default()).await;
    let (manifest, _) = verify(
        baseline.process(source.clone(), &recipe).await,
        &source,
        &recipe,
    )
    .await;
    baseline.released().await;
    let sum = manifest.outputs.iter().map(|o| o.byte_size).sum::<u64>();
    for (bytes, status) in [(sum, 200), (sum - 1, 413)] {
        let server = Server::start(Limits {
            output_bytes: bytes,
            ..Limits::default()
        })
        .await;
        assert_eq!(
            server.process(source.clone(), &recipe).await.status(),
            status
        );
        server.released().await;
    }
}
use application::images::{Limits, SourceFormat};
use image::{DynamicImage, ImageFormat};
use std::io::Cursor;
use support::*;

fn apng(hidden_poster: bool, frames: u32, loops: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_animated(frames, loops).unwrap();
        encoder.set_sep_def_img(hidden_poster).unwrap();
        encoder.set_frame_delay(1, 60).unwrap();
        let mut writer = encoder.write_header().unwrap();
        if hidden_poster {
            writer
                .write_image_data(&[0, 0, 0, 255, 0, 0, 0, 255])
                .unwrap();
        }
        writer
            .write_image_data(&[255, 0, 0, 255, 0, 0, 255, 255])
            .unwrap();
        if frames > 1 {
            writer.set_frame_dimension(1, 1).unwrap();
            writer.set_frame_position(0, 0).unwrap();
            writer.set_blend_op(png::BlendOp::Over).unwrap();
            writer.set_dispose_op(png::DisposeOp::Previous).unwrap();
            writer.write_image_data(&[0, 255, 0, 128]).unwrap();
            writer.set_frame_position(1, 0).unwrap();
            writer.set_blend_op(png::BlendOp::Source).unwrap();
            writer.set_dispose_op(png::DisposeOp::Background).unwrap();
            writer.write_image_data(&[255, 255, 255, 255]).unwrap();
        }
        writer.finish().unwrap();
    }
    bytes
}

#[tokio::test]
async fn apng_preserves_composited_pixels_fractional_timing_loops_and_hidden_default_image() {
    let server = Server::start(Limits::default()).await;
    for hidden in [false, true] {
        let source = apng(hidden, 3, 4);
        let recipe = recipe("lossless", "preserve", &[(2, 1), (1, 1)]);
        let (manifest, outputs) = verify(
            server.process(source.clone(), &recipe).await,
            &source,
            &recipe,
        )
        .await;
        assert_eq!(manifest.source.format, SourceFormat::Apng);
        assert!(manifest.source.image.animated);
        assert_eq!(manifest.source.image.frame_count, 3);
        assert_eq!(manifest.source.image.duration_ms, 50);
        assert_eq!(manifest.source.image.loop_count, 4);
        let mut frames = Vec::new();
        let mut delays = Vec::new();
        image_codec::decode_frames(&outputs[0], &Limits::default(), |pixels, delay| {
            frames.push(pixels);
            delays.push(delay);
            Ok(())
        })
        .unwrap();
        assert_eq!(delays, [17, 16, 17]);
        assert_eq!(frames[0], [255, 0, 0, 255, 0, 0, 255, 255]);
        assert_eq!(frames[1][3], 255);
        assert!(frames[1][0].abs_diff(127) <= 1 && frames[1][1].abs_diff(128) <= 1);
        assert_eq!(&frames[1][4..], &[0, 0, 255, 255]);
        assert_eq!(frames[2], [255, 0, 0, 255, 255, 255, 255, 255]);
        for output in &manifest.outputs {
            assert!(output.image.animated);
            assert_eq!(output.image.frame_count, 3);
        }
        server.released().await;
    }
}

#[tokio::test]
async fn single_frame_animation_keeps_duration_loop_and_animation_container_on_roundtrip() {
    let server = Server::start(Limits::default()).await;
    let source = apng(false, 1, 7);
    let recipe = recipe("lossless", "preserve", &[(2, 1), (1, 1)]);
    let (manifest, outputs) = verify(
        server.process(source.clone(), &recipe).await,
        &source,
        &recipe,
    )
    .await;
    for output in &manifest.outputs {
        assert!(output.image.animated);
        assert_eq!(output.image.frame_count, 1);
        assert_eq!(output.image.duration_ms, 17);
        assert_eq!(output.image.loop_count, 7);
    }
    server.released().await;
    verify(
        server.process(outputs[0].clone(), &recipe).await,
        &outputs[0],
        &recipe,
    )
    .await;
    server.released().await;
}

#[tokio::test]
async fn bitmap_ico_tiff_qoi_pnm_and_sixteen_bit_png_produce_multiple_verified_sizes() {
    let server = Server::start(Limits::default()).await;
    let image = image::RgbImage::from_fn(8, 4, |x, y| {
        image::Rgb([(x * 30) as u8, (y * 60) as u8, 120])
    });
    for (format, expected) in [
        (ImageFormat::Bmp, SourceFormat::Bmp),
        (ImageFormat::Ico, SourceFormat::Ico),
        (ImageFormat::Tiff, SourceFormat::Tiff),
        (ImageFormat::Qoi, SourceFormat::Qoi),
        (ImageFormat::Pnm, SourceFormat::Pnm),
    ] {
        let mut source = Cursor::new(Vec::new());
        let pixels = if format == ImageFormat::Ico {
            DynamicImage::ImageRgba8(DynamicImage::ImageRgb8(image.clone()).into_rgba8())
        } else {
            DynamicImage::ImageRgb8(image.clone())
        };
        pixels.write_to(&mut source, format).unwrap();
        let source = source.into_inner();
        let recipe = recipe("lossless", "preserve", &[(8, 4), (4, 2), (2, 1)]);
        let (manifest, _) = verify(
            server.process(source.clone(), &recipe).await,
            &source,
            &recipe,
        )
        .await;
        assert_eq!(manifest.source.format, expected);
        assert!(!manifest.source.image.animated);
        assert_eq!(manifest.outputs.len(), 3);
        server.released().await;
    }
    let pixels = image::ImageBuffer::<image::Rgba<u16>, _>::from_pixel(
        8,
        4,
        image::Rgba([65535, 32896, 0, 32896]),
    );
    let mut source = Cursor::new(Vec::new());
    DynamicImage::ImageRgba16(pixels)
        .write_to(&mut source, ImageFormat::Png)
        .unwrap();
    let source = source.into_inner();
    let recipe = recipe("lossless", "preserve", &[(8, 4), (4, 2)]);
    let (_, outputs) = verify(
        server.process(source.clone(), &recipe).await,
        &source,
        &recipe,
    )
    .await;
    image_codec::decode_frames(&outputs[0], &Limits::default(), |pixels, _| {
        assert!(
            pixels
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| *p == [255, 128, 0, 128])
        );
        Ok(())
    })
    .unwrap();
    server.released().await;
}

#[tokio::test]
async fn heic_avif_jxl_and_cmyk_jpeg_decode_real_encoded_fixtures() {
    let server = Server::start(Limits::default()).await;
    for (name, format) in [
        ("red-static.avif", SourceFormat::Avif),
        ("red-static.jxl", SourceFormat::Jxl),
        ("green-alpha.heic", SourceFormat::Heic),
        ("red-cmyk.jpg", SourceFormat::Jpeg),
        ("red-cmyk-icc.jpg", SourceFormat::Jpeg),
    ] {
        let source = fixture(name);
        let recipe = recipe("lossless", "preserve", &[(8, 4), (4, 2), (2, 1)]);
        let (manifest, outputs) = verify(
            server.process(source.clone(), &recipe).await,
            &source,
            &recipe,
        )
        .await;
        assert_eq!(manifest.source.format, format, "{name}");
        assert!(!manifest.source.image.animated);
        image_codec::decode_frames(&outputs[0], &Limits::default(), |pixels, _| {
            let expected = if name.ends_with(".heic") {
                [0, 255, 0, 128]
            } else {
                [255, 0, 0, 255]
            };
            assert!(
                pixels
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .all(|p| p.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 4)),
                "{name}: {:?}",
                &pixels[..4]
            );
            Ok(())
        })
        .unwrap();
        server.released().await;
    }
}

#[tokio::test]
async fn avif_and_jxl_animations_keep_all_frames_alpha_timing_and_loops_in_every_size() {
    let server = Server::start(Limits::default()).await;
    for (name, delays, loops) in [
        ("colors-animated.avif", vec![100, 200, 300], 3),
        ("colors-animated.jxl", vec![100, 100, 100], 0),
    ] {
        let source = fixture(name);
        let recipe = recipe("lossless", "preserve", &[(8, 4), (4, 2), (2, 1)]);
        let (manifest, outputs) = verify(
            server.process(source.clone(), &recipe).await,
            &source,
            &recipe,
        )
        .await;
        assert_eq!(manifest.source.image.frame_count, 3, "{name}");
        for output in &outputs {
            let mut actual_delays = Vec::new();
            let mut index = 0;
            let facts = image_codec::decode_frames(output, &Limits::default(), |pixels, delay| {
                let expected = [[255, 0, 0, 255], [0, 255, 0, 128], [0, 0, 255, 255]][index];
                assert!(
                    pixels
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .all(|p| p.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 4)),
                    "{name} frame {index}: {:?}",
                    &pixels[..4]
                );
                actual_delays.push(delay);
                index += 1;
                Ok(())
            })
            .unwrap();
            assert!(facts.animated);
            assert_eq!(facts.loop_count, loops, "{name}");
            assert_eq!(actual_delays, delays, "{name}");
        }
        server.released().await;
    }
}

#[tokio::test]
async fn static_svg_renders_straight_alpha_and_unsupported_active_content_fails_explicitly() {
    let server = Server::start(Limits::default()).await;
    let source = br#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="4"><rect width="8" height="4" fill="red" fill-opacity="0.5"/></svg>"#.to_vec();
    let recipe = recipe("lossless", "preserve", &[(8, 4), (4, 2)]);
    let (manifest, outputs) = verify(
        server.process(source.clone(), &recipe).await,
        &source,
        &recipe,
    )
    .await;
    assert_eq!(manifest.source.format, SourceFormat::Svg);
    image_codec::decode_frames(&outputs[0], &Limits::default(), |pixels, _| {
        assert!(
            pixels
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| p[0] == 255 && p[1] == 0 && p[2] == 0 && p[3].abs_diff(128) <= 1)
        );
        Ok(())
    })
    .unwrap();
    server.released().await;
    for fragment in [
        r#"<animate attributeName="x" values="0;2" dur="1s"/>"#,
        "<script>ignored()</script>",
        r#"<style>@keyframes spin {to {opacity:0}}</style>"#,
    ] {
        let source = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="4">{fragment}</svg>"#
        );
        let response = server.process(source.into_bytes(), &recipe).await;
        assert_eq!(response.status(), 422);
        assert_eq!(
            response.json::<serde_json::Value>().await.unwrap()["error"]["code"],
            "unrepresentable_animation"
        );
        server.released().await;
    }
    let source = br#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="4"><image href="file:///etc/passwd"/></svg>"#.to_vec();
    assert_eq!(server.process(source, &recipe).await.status(), 415);
    server.released().await;
}

#[tokio::test]
async fn corrupt_late_apng_frame_does_not_return_partial_outputs() {
    let server = Server::start(Limits::default()).await;
    let mut source = apng(false, 3, 0);
    let mut cursor = 8;
    let mut last = None;
    while cursor < source.len() {
        let length = u32::from_be_bytes(source[cursor..cursor + 4].try_into().unwrap()) as usize;
        if &source[cursor + 4..cursor + 8] == b"fdAT" {
            last = Some((cursor, length));
        }
        cursor += length + 12;
    }
    let (cursor, length) = last.unwrap();
    source[cursor + 12] = 0;
    let crc = crc32fast::hash(&source[cursor + 4..cursor + 8 + length]);
    source[cursor + 8 + length..cursor + 12 + length].copy_from_slice(&crc.to_be_bytes());
    let response = server
        .process(source, &recipe("lossless", "preserve", &[(2, 1), (1, 1)]))
        .await;
    assert_eq!(response.status(), 422);
    assert_eq!(
        response.json::<serde_json::Value>().await.unwrap()["error"]["code"],
        "invalid_image"
    );
    server.released().await;
}
