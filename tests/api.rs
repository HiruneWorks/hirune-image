mod support;
use application::images::Limits;
use support::*;

#[tokio::test]
async fn static_png_and_jpeg_have_verified_variants_and_raw_recipe_binding() {
    let server = Server::start(Limits::default()).await;
    for source in [
        png(97, 53),
        fixture("srgb-orientation6-gps.jpg"),
        fixture("p3-orientation6-gps.jpg"),
    ] {
        for mode in ["lossy", "lossless"] {
            let recipe = format!(
                " \n{}\n",
                recipe(mode, "preserve", &[(4096, 4096), (32, 32), (16, 8)])
            );
            let response = server.process(source.clone(), &recipe).await;
            let (manifest, _) = verify(response, &source, &recipe).await;
            assert_eq!(manifest.outputs.len(), 3);
            if manifest.source.orientation == 6 {
                assert_eq!(
                    (
                        manifest.outputs[0].image.width,
                        manifest.outputs[0].image.height
                    ),
                    (40, 84)
                );
            } else {
                assert_eq!(
                    (
                        manifest.outputs[0].image.width,
                        manifest.outputs[0].image.height
                    ),
                    (97, 53)
                );
            }
            server.released().await;
        }
    }
}
#[tokio::test]
async fn animated_gif_and_webp_preserve_timing_loops_disposal_and_pixels() {
    let server = Server::start(Limits::default()).await;
    for (name, loops, duration, frames) in [
        ("finite1.gif", 2, 1100, 3),
        ("finite3.gif", 4, 1100, 3),
        ("infinite.gif", 0, 1100, 3),
        ("missing-loop.gif", 1, 1100, 3),
        ("coalesced.gif", 4, 1100, 3),
        ("disposal.gif", 4, 1600, 4),
        ("rust-finite1-lossless.webp", 2, 1100, 3),
        ("rust-disposal-lossless.webp", 4, 1600, 4),
    ] {
        let source = fixture(name);
        let recipe = recipe("lossless", "preserve", &[(4096, 4096), (4, 4)]);
        let (manifest, outputs) = verify(
            server.process(source.clone(), &recipe).await,
            &source,
            &recipe,
        )
        .await;
        assert_eq!(manifest.source.image.frame_count, frames);
        for output in &manifest.outputs {
            assert_eq!(output.image.duration_ms, duration);
            assert_eq!(output.image.loop_count, loops);
        }
        if name.ends_with(".gif") {
            let stem = name.trim_end_matches(".gif");
            let mut i = 0;
            image_codec::decode_frames(&outputs[0], &Limits::default(), |actual, _| {
                let expected = fixture(&format!("{stem}-expected-{i}.rgba"));
                assert_eq!(actual.len(), expected.len());
                for (a, b) in actual
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(expected.as_chunks::<4>().0.iter())
                {
                    assert_eq!(a[3], b[3]);
                    if b[3] > 0 {
                        assert_eq!(&a[..3], &b[..3]);
                    }
                }
                i += 1;
                Ok(())
            })
            .unwrap();
            assert_eq!(i, frames);
        }
        server.released().await;
    }
}
#[tokio::test]
async fn animation_policies_and_strict_recipe_rejections() {
    let server = Server::start(Limits::default()).await;
    let source = fixture("finite1.gif");
    let first = recipe("lossy", "first_frame", &[(16, 16)]);
    let (manifest, _) = verify(
        server.process(source.clone(), &first).await,
        &source,
        &first,
    )
    .await;
    assert_eq!(manifest.source.image.frame_count, 3);
    assert_eq!(manifest.outputs[0].image.duration_ms, 0);
    assert_eq!(manifest.outputs[0].image.loop_count, 1);
    let reject = recipe("lossless", "reject", &[(16, 16)]);
    let response = server.process(source.clone(), &reject).await;
    assert_eq!(response.status(), 422);
    assert_eq!(
        response.json::<serde_json::Value>().await.unwrap()["error"]["code"],
        "animation_not_allowed"
    );
    let valid = recipe("lossless", "preserve", &[(16, 16)]);
    for invalid in [
        valid.replace(
            "\"contract_version\":1",
            "\"contract_version\":1,\"contract_version\":1",
        ),
        valid.replace("\"effort\":0", "\"effort\":0,\"effort\":0"),
        valid.replace(
            "\"mode\":\"lossless\"",
            "\"mode\":\"lossless\",\"quality\":85",
        ),
        valid.replace("\"fit\":\"inside\"", "\"fit\":\"cover\""),
        valid.replace("\"max_width\":16", "\"max_width\":0"),
        valid.replace("\"effort\":0", "\"effort\":7"),
        valid.replace("\"contract_version\":1", "\"contract_version\":2"),
    ] {
        let response = server.process(source.clone(), &invalid).await;
        assert_eq!(response.status(), 400, "invalid recipe accepted: {invalid}");
        assert_eq!(
            response.json::<serde_json::Value>().await.unwrap()["error"]["code"],
            "invalid_recipe"
        );
    }
    server.released().await;
}
#[tokio::test]
async fn unsupported_truncated_and_trailing_sources_fail_closed() {
    let server = Server::start(Limits::default()).await;
    let recipe = recipe("lossless", "preserve", &[(32, 32)]);
    assert_eq!(
        server.process(b"<svg/>".to_vec(), &recipe).await.status(),
        415
    );
    for mut source in [
        png(8, 8),
        fixture("finite1.gif"),
        fixture("srgb-orientation6-gps.jpg"),
        fixture("rust-finite1-lossless.webp"),
    ] {
        source.push(0);
        assert_eq!(server.process(source.clone(), &recipe).await.status(), 422);
        source.truncate(source.len() / 2);
        assert_eq!(server.process(source, &recipe).await.status(), 422);
    }
    server.released().await;
}
#[tokio::test]
async fn source_and_output_budgets_are_enforced_and_capabilities_are_effective() {
    let limits = Limits {
        canvas_pixels: 64,
        output_bytes: 10,
        ..Limits::default()
    };
    let server = Server::start(limits).await;
    let recipe = recipe("lossless", "preserve", &[(8, 8)]);
    for (source, code) in [
        (png(9, 8), "image_limits_exceeded"),
        (png(8, 8), "output_limits_exceeded"),
    ] {
        let response = server.process(source, &recipe).await;
        assert_eq!(response.status(), 413);
        assert_eq!(
            response.json::<serde_json::Value>().await.unwrap()["error"]["code"],
            code
        );
    }
    let capability: serde_json::Value = server
        .client
        .get(format!("{}/v1/capabilities", server.url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(capability["limits"]["canvas_pixels"], 64);
    assert_eq!(capability["limits"]["output_bytes"], 10);
    server.released().await;
}

#[tokio::test]
async fn duplicate_unknown_parts_and_repeated_closing_boundary_are_rejected() {
    let server = Server::start(Limits::default()).await;
    let source = png(2, 2);
    let recipe = recipe("lossless", "preserve", &[(2, 2)]);
    let valid = raw_multipart(&source, &recipe);
    let mut trailing = valid.clone();
    trailing.extend_from_slice(b"junk\r\n--strict--\r\n");
    let duplicate=valid[..valid.len()-14].iter().copied().chain(b"\r\n--strict\r\nContent-Disposition: form-data; name=\"recipe\"\r\nContent-Type: application/json\r\n\r\n{}\r\n--strict--\r\n".iter().copied()).collect();
    let unknown = String::from_utf8_lossy(&valid)
        .replace("name=\"recipe\"", "name=\"unknown\"")
        .into_bytes();
    for body in [trailing, duplicate, unknown] {
        let response = server
            .client
            .post(format!("{}/v1/images/process", server.url))
            .header("content-type", "multipart/form-data; boundary=strict")
            .body(body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 400);
    }
    server.released().await;
}
#[tokio::test]
async fn exif_orientations_one_through_eight_change_real_pixels_and_strip_metadata() {
    use image::ImageEncoder;
    let server = Server::start(Limits::default()).await;
    let image = image::RgbImage::from_fn(12, 8, |x, y| {
        image::Rgb([(x * 20) as u8, (y * 30) as u8, 80])
    });
    for orientation in 1u8..=8 {
        let mut source = Vec::new();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut source, 95);
        let mut exif =
            b"II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x01\0\0\0\0\0\0\0".to_vec();
        exif[18] = orientation;
        encoder.set_exif_metadata(exif).unwrap();
        encoder.encode_image(&image).unwrap();
        let mut expected = image::load_from_memory(&source).unwrap();
        expected.apply_orientation(image::metadata::Orientation::from_exif(orientation).unwrap());
        let recipe = recipe("lossless", "preserve", &[(32, 32)]);
        let (manifest, outputs) = verify(
            server.process(source.clone(), &recipe).await,
            &source,
            &recipe,
        )
        .await;
        assert_eq!(manifest.source.orientation, orientation);
        assert_eq!(
            (
                manifest.outputs[0].image.width,
                manifest.outputs[0].image.height
            ),
            (expected.width(), expected.height())
        );
        image_codec::decode_frames(&outputs[0], &Limits::default(), |actual, _| {
            assert_eq!(actual, expected.to_rgba8().into_raw());
            Ok(())
        })
        .unwrap();
    }
}
#[tokio::test]
async fn palette_png_and_malformed_profiles_have_explicit_behavior() {
    let server = Server::start(Limits::default()).await;
    let recipe = recipe("lossless", "preserve", &[(16, 16)]);
    let mut source = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut source, 8, 1);
        encoder.set_color(png::ColorType::Indexed);
        encoder.set_depth(png::BitDepth::One);
        encoder.set_palette(vec![255, 0, 0, 0, 255, 0]);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[0b01010101]).unwrap();
    }
    verify(
        server.process(source.clone(), &recipe).await,
        &source,
        &recipe,
    )
    .await;
    let mut source = fixture("p3-orientation6-gps.jpg");
    let signature = source.windows(4).position(|s| s == b"acsp").unwrap();
    source[signature..signature + 4].copy_from_slice(b"oops");
    let response = server.process(source, &recipe).await;
    assert_eq!(response.status(), 422);
    assert_eq!(
        response.json::<serde_json::Value>().await.unwrap()["error"]["code"],
        "unsupported_color_profile"
    );
    server.released().await;
}

#[tokio::test]
async fn zero_delay_loop_overflow_and_late_webp_corruption_are_rejected() {
    let server = Server::start(Limits::default()).await;
    let recipe = recipe("lossless", "preserve", &[(32, 32)]);
    let source = fixture("finite1.gif");
    let mut zero = source.clone();
    let gce = zero.windows(3).position(|s| s == [0x21, 0xf9, 4]).unwrap();
    zero[gce + 4] = 0;
    zero[gce + 5] = 0;
    let mut overflow = source.clone();
    let app = overflow
        .windows(11)
        .position(|s| s == b"NETSCAPE2.0")
        .unwrap();
    overflow[app + 13] = 255;
    overflow[app + 14] = 255;
    for source in [zero, overflow] {
        let response = server.process(source, &recipe).await;
        assert_eq!(response.status(), 422);
        assert_eq!(
            response.json::<serde_json::Value>().await.unwrap()["error"]["code"],
            "unrepresentable_animation"
        );
    }
    let mut broken = fixture("rust-finite1-lossless.webp");
    let last = broken.windows(4).rposition(|s| s == b"VP8L").unwrap();
    broken[last + 8] = 0;
    assert_eq!(server.process(broken, &recipe).await.status(), 422);
    server.released().await;
}
#[tokio::test]
async fn icc_declared_length_must_match_actual_profile() {
    let server = Server::start(Limits::default()).await;
    let recipe = recipe("lossless", "preserve", &[(32, 32)]);
    let source = fixture("p3-orientation6-gps.jpg");
    let start = source
        .windows(12)
        .position(|s| s == b"ICC_PROFILE\0")
        .unwrap()
        + 14;
    for declared in [0u32, 1, 999999] {
        let mut broken = source.clone();
        broken[start..start + 4].copy_from_slice(&declared.to_be_bytes());
        let response = server.process(broken, &recipe).await;
        assert_eq!(response.status(), 422);
        assert_eq!(
            response.json::<serde_json::Value>().await.unwrap()["error"]["code"],
            "unsupported_color_profile"
        );
    }
    server.released().await;
}
#[tokio::test]
async fn independent_response_verifier_rejects_tampering_truncation_and_extra_data() {
    let server = Server::start(Limits::default()).await;
    let source = png(8, 8);
    let recipe = recipe("lossless", "preserve", &[(8, 8), (4, 4)]);
    let response = server.process(source.clone(), &recipe).await;
    assert_eq!(response.status(), 200);
    let body = response.bytes().await.unwrap();
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("source"), &source).unwrap();
    std::fs::write(directory.path().join("recipe"), recipe).unwrap();
    let mut tampered = body.to_vec();
    let riff = tampered.windows(4).position(|w| w == b"RIFF").unwrap();
    tampered[riff] = 0;
    let mut trailing = body.to_vec();
    trailing.extend_from_slice(b"garbage");
    for (bytes, valid) in [
        (body.to_vec(), true),
        (tampered, false),
        (trailing, false),
        (body[..body.len() / 2].to_vec(), false),
    ] {
        std::fs::write(directory.path().join("response"), bytes).unwrap();
        let interpreter = if cfg!(windows) { "python" } else { "python3" };
        let output = std::process::Command::new(interpreter)
            .arg("scripts/verify_response.py")
            .arg(directory.path().join("response"))
            .arg("--source")
            .arg(directory.path().join("source"))
            .arg("--recipe")
            .arg(directory.path().join("recipe"))
            .output()
            .unwrap();
        assert_eq!(
            output.status.success(),
            valid,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    server.released().await;
}
#[tokio::test]
async fn actual_byte_limits_apply_to_chunked_requests_and_exact_source_size() {
    let source = png(8, 8);
    let recipe = recipe("lossless", "preserve", &[(8, 8)]);
    let server = Server::start(Limits {
        source_bytes: source.len() as u64,
        ..Limits::default()
    })
    .await;
    verify(
        server.process(source.clone(), &recipe).await,
        &source,
        &recipe,
    )
    .await;
    let mut over = source.clone();
    over.push(0);
    assert_eq!(server.process(over, &recipe).await.status(), 413);
    let limits = Limits {
        source_bytes: 128,
        recipe_bytes: 512,
        request_bytes: 1664,
        ..Limits::default()
    };
    let small = Server::start(limits).await;
    let chunks =
        futures_util::stream::iter([Ok::<_, std::io::Error>(bytes::Bytes::from(vec![0; 2048]))]);
    let response = small
        .client
        .post(format!("{}/v1/images/process", small.url))
        .header("content-type", "multipart/form-data; boundary=strict")
        .body(reqwest::Body::wrap_stream(chunks))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 413);
    small.released().await;
}

#[tokio::test]
async fn exact_static_and_animation_output_byte_caps_are_accepted() {
    let baseline = Server::start(Limits::default()).await;
    for source in [png(8, 8), fixture("finite1.gif")] {
        let recipe = recipe("lossless", "preserve", &[(4096, 4096)]);
        let (manifest, _) = verify(
            baseline.process(source.clone(), &recipe).await,
            &source,
            &recipe,
        )
        .await;
        let exact = manifest.outputs[0].byte_size;
        let server = Server::start(Limits {
            output_bytes: exact,
            ..Limits::default()
        })
        .await;
        verify(
            server.process(source.clone(), &recipe).await,
            &source,
            &recipe,
        )
        .await;
        server.released().await;
        let smaller = Server::start(Limits {
            output_bytes: exact - 1,
            ..Limits::default()
        })
        .await;
        assert_eq!(smaller.process(source, &recipe).await.status(), 413);
        smaller.released().await;
    }
}
