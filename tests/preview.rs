mod support;
use application::{
    error::ProcessingError,
    images::{
        AnimationPolicy, DecodeMode, Encoding, JobInput, Limits, OutputRecipe, Recipe,
        inside_dimensions, sha256,
    },
};
use image::{ExtendedColorType, Rgb, RgbImage};

fn jpeg(width: u32, height: u32) -> Vec<u8> {
    let pixels = RgbImage::from_fn(width, height, |x, y| {
        Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8])
    });
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 94)
        .encode(pixels.as_raw(), width, height, ExtendedColorType::Rgb8)
        .unwrap();
    bytes
}

fn orient(bytes: &[u8], value: u16) -> Vec<u8> {
    let mut exif = b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0".to_vec();
    exif.extend(value.to_le_bytes());
    exif.extend([0; 6]);
    let mut result = bytes[..2].to_vec();
    result.extend([0xff, 0xe1]);
    result.extend(((exif.len() + 2) as u16).to_be_bytes());
    result.extend(exif);
    result.extend(&bytes[2..]);
    result
}

fn process(
    bytes: &[u8],
    mode: DecodeMode,
    boxes: &[(u32, u32)],
    limits: Limits,
) -> Result<
    (
        application::images::Manifest,
        serde_json::Value,
        Vec<Vec<u8>>,
    ),
    ProcessingError,
> {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("source"), bytes).unwrap();
    let recipe = Recipe {
        animation_policy: AnimationPolicy::Preserve,
        decode_mode: mode,
        outputs: boxes
            .iter()
            .enumerate()
            .map(|(index, &(max_width, max_height))| OutputRecipe {
                id: format!("size_{index}"),
                max_width,
                max_height,
                encoding: Encoding::Lossless { effort: 0 },
            })
            .collect(),
    };
    let manifest = image_codec::process_job(
        directory.path(),
        &JobInput {
            recipe,
            limits,
            source_sha256: sha256(bytes),
            recipe_sha256: sha256(b"preview regression"),
        },
    )?;
    let metrics =
        serde_json::from_slice(&std::fs::read(directory.path().join("metrics.json")).unwrap())
            .unwrap();
    let outputs = (0..boxes.len())
        .map(|index| {
            std::fs::read(directory.path().join(format!("output-size_{index}.webp"))).unwrap()
        })
        .collect();
    Ok((manifest, metrics, outputs))
}

#[test]
fn preview_preserves_original_facts_and_geometry_for_baseline_progressive_and_all_orientations() {
    for bytes in [jpeg(257, 131), support::fixture("preview-progressive.jpg")] {
        for orientation in 1..=8 {
            let source = orient(&bytes, orientation);
            let boxes = [(33, 33), (17, 13), (9, 7)];
            let (accurate, _, _) =
                process(&source, DecodeMode::Accurate, &boxes, Limits::default()).unwrap();
            let (preview, metrics, _) =
                process(&source, DecodeMode::Preview, &boxes, Limits::default()).unwrap();
            assert_eq!(preview.source.image, accurate.source.image);
            assert_eq!(preview.source.orientation, orientation as u8);
            assert_eq!(preview.source.byte_size, source.len() as u64);
            assert_eq!(preview.source_sha256, sha256(&source));
            let display = if orientation >= 5 {
                (131, 257)
            } else {
                (257, 131)
            };
            assert!(metrics["decoded_width"].as_u64().unwrap() < display.0 as u64);
            assert!(metrics["decoded_height"].as_u64().unwrap() < display.1 as u64);
            assert_eq!(metrics["source_decoder_starts"], 1);
            for (output, &(w, h)) in preview.outputs.iter().zip(&boxes) {
                assert_eq!(
                    (output.image.width, output.image.height),
                    inside_dimensions(display.0, display.1, w, h)
                );
            }
        }
    }
}

#[test]
fn largest_output_icc_cmyk_and_non_jpeg_keep_the_accurate_path() {
    for bytes in [
        jpeg(257, 131),
        support::fixture("red-cmyk.jpg"),
        support::fixture("red-cmyk-icc.jpg"),
        support::fixture("p3-orientation6-gps.jpg"),
        support::png(71, 43),
        support::fixture("finite1.gif"),
    ] {
        let boxes = [(7, 7), (4096, 4096)];
        let (_, accurate, accurate_outputs) =
            process(&bytes, DecodeMode::Accurate, &boxes, Limits::default()).unwrap();
        let (_, preview, preview_outputs) =
            process(&bytes, DecodeMode::Preview, &boxes, Limits::default()).unwrap();
        assert_eq!(preview["decoded_width"], accurate["decoded_width"]);
        assert_eq!(preview["decoded_height"], accurate["decoded_height"]);
        assert_eq!(preview_outputs, accurate_outputs);
    }
    // Small requests alone still retain the accurate decoder for ICC and CMYK sources.
    for file in [
        "red-cmyk.jpg",
        "red-cmyk-icc.jpg",
        "p3-orientation6-gps.jpg",
    ] {
        let source = support::fixture(file);
        let (_, accurate, outputs) =
            process(&source, DecodeMode::Accurate, &[(1, 1)], Limits::default()).unwrap();
        let (_, preview, actual) =
            process(&source, DecodeMode::Preview, &[(1, 1)], Limits::default()).unwrap();
        assert_eq!(preview["decoded_width"], accurate["decoded_width"]);
        assert_eq!(preview["decoded_height"], accurate["decoded_height"]);
        assert_eq!(actual, outputs);
    }
}

#[test]
fn reduction_does_not_bypass_original_limits_and_handles_one_pixel_sides() {
    let source = jpeg(257, 131);
    let pixels = 257 * 131;
    for limits in [
        Limits {
            decoded_bytes: pixels * 4 - 1,
            ..Limits::default()
        },
        Limits {
            working_bytes: pixels * 20 - 1,
            ..Limits::default()
        },
        Limits {
            canvas_pixels: pixels - 1,
            ..Limits::default()
        },
        Limits {
            input_dimension: 256,
            ..Limits::default()
        },
    ] {
        assert!(matches!(
            process(&source, DecodeMode::Preview, &[(7, 7)], limits),
            Err(ProcessingError::ImageLimitsExceeded)
        ));
    }
    for dimensions in [(257, 1), (1, 257)] {
        let (manifest, metrics, _) = process(
            &jpeg(dimensions.0, dimensions.1),
            DecodeMode::Preview,
            &[(17, 17)],
            Limits::default(),
        )
        .unwrap();
        assert_eq!(
            (
                manifest.outputs[0].image.width,
                manifest.outputs[0].image.height
            ),
            inside_dimensions(dimensions.0, dimensions.1, 17, 17)
        );
        assert!(
            metrics["decoded_width"].as_u64().unwrap()
                * metrics["decoded_height"].as_u64().unwrap()
                < 257
        );
    }
}

#[test]
fn corrupt_jpeg_tail_is_never_returned_as_a_successful_preview() {
    let mut source = support::fixture("preview-progressive.jpg");
    source.truncate(source.len() - 24);
    assert!(process(&source, DecodeMode::Preview, &[(7, 7)], Limits::default()).is_err());
}

#[test]
fn progressive_coefficient_padding_falls_back_before_exceeding_the_working_plan() {
    let source = support::fixture("preview-progressive-thin.jpg");
    let limits = Limits {
        working_bytes: 257 * 20,
        ..Limits::default()
    };
    let (_, accurate, expected) =
        process(&source, DecodeMode::Accurate, &[(17, 17)], limits.clone()).unwrap();
    let (_, preview, actual) = process(&source, DecodeMode::Preview, &[(17, 17)], limits).unwrap();
    assert_eq!(preview["decoded_width"], accurate["decoded_width"]);
    assert_eq!(preview["decoded_height"], accurate["decoded_height"]);
    assert_eq!(actual, expected);
    let (_, reduced, _) =
        process(&source, DecodeMode::Preview, &[(17, 17)], Limits::default()).unwrap();
    assert!(reduced["decoded_width"].as_u64().unwrap() < 257);
}

#[test]
fn all_preview_exif_transforms_put_asymmetric_colors_in_the_correct_corners() {
    let colors = [[220, 20, 20], [20, 220, 20], [20, 20, 220], [220, 220, 20]];
    let pixels = RgbImage::from_fn(257, 131, |x, y| {
        Rgb(colors[usize::from(y >= 65) * 2 + usize::from(x >= 128)])
    });
    let mut baseline = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut baseline, 94)
        .encode(pixels.as_raw(), 257, 131, ExtendedColorType::Rgb8)
        .unwrap();
    let expected_corners = [
        [0, 1, 2, 3],
        [1, 0, 3, 2],
        [3, 2, 1, 0],
        [2, 3, 0, 1],
        [0, 2, 1, 3],
        [2, 0, 3, 1],
        [3, 1, 2, 0],
        [1, 3, 0, 2],
    ];
    for source in [baseline, support::fixture("preview-progressive.jpg")] {
        for (index, expected) in expected_corners.iter().enumerate() {
            let source = orient(&source, (index + 1) as u16);
            let (manifest, _, outputs) =
                process(&source, DecodeMode::Preview, &[(33, 33)], Limits::default()).unwrap();
            let (width, height) = (
                manifest.outputs[0].image.width,
                manifest.outputs[0].image.height,
            );
            let mut actual = None;
            image_codec::decode_frames(&outputs[0], &Limits::default(), |pixels, _| {
                actual = image::RgbaImage::from_raw(width, height, pixels);
                Ok(())
            })
            .unwrap();
            let actual = actual.unwrap();
            for ((x, y), &color) in [
                (width / 4, height / 4),
                (width * 3 / 4, height / 4),
                (width / 4, height * 3 / 4),
                (width * 3 / 4, height * 3 / 4),
            ]
            .iter()
            .zip(expected)
            {
                let pixel = actual.get_pixel(*x, *y);
                assert_eq!(pixel[3], 255);
                for channel in 0..3 {
                    assert!(
                        pixel[channel].abs_diff(colors[color][channel]) <= 8,
                        "orientation {} corner ({x}, {y}): {pixel:?}",
                        index + 1
                    );
                }
            }
        }
    }
}

#[test]
fn grayscale_preview_keeps_equal_channels() {
    let pixels = image::GrayImage::from_fn(257, 131, |x, y| image::Luma([((x + y) % 256) as u8]));
    let mut source = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut source, 94)
        .encode(pixels.as_raw(), 257, 131, ExtendedColorType::L8)
        .unwrap();
    let (_, metrics, outputs) =
        process(&source, DecodeMode::Preview, &[(33, 33)], Limits::default()).unwrap();
    assert!(metrics["decoded_width"].as_u64().unwrap() < 257);
    image_codec::decode_frames(&outputs[0], &Limits::default(), |pixels, _| {
        assert!(
            pixels
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| p[0] == p[1] && p[1] == p[2] && p[3] == 255)
        );
        Ok(())
    })
    .unwrap();
}

#[tokio::test]
async fn http_omission_defaults_to_accurate_and_preview_is_typed_and_verified() {
    let server = support::Server::start(Limits::default()).await;
    let source = jpeg(257, 131);
    let original = support::recipe("lossless", "preserve", &[(33, 33), (9, 7)]);
    let (_, accurate) = support::verify(
        server.process(source.clone(), &original).await,
        &source,
        &original,
    )
    .await;
    server.released().await;
    for mode in ["accurate", "preview"] {
        let mut recipe: serde_json::Value = serde_json::from_str(&original).unwrap();
        recipe["decode_mode"] = serde_json::json!(mode);
        let recipe = recipe.to_string();
        let (manifest, outputs) = support::verify(
            server.process(source.clone(), &recipe).await,
            &source,
            &recipe,
        )
        .await;
        assert_eq!(
            (
                manifest.outputs[0].image.width,
                manifest.outputs[0].image.height
            ),
            (33, 16)
        );
        if mode == "accurate" {
            assert_eq!(outputs, accurate);
        }
        server.released().await;
    }
    for field in [
        "\"decode_mode\":\"unknown\",",
        "\"decode_mode\":\"accurate\",\"decode_mode\":\"preview\",",
    ] {
        let recipe = original.replacen('{', &format!("{{{field}"), 1);
        assert_eq!(server.process(source.clone(), &recipe).await.status(), 400);
        server.released().await;
    }
}
