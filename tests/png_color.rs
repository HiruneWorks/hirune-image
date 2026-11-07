mod support;

use application::images::Limits;
use support::{Server, recipe, verify};

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

fn source(depth: png::BitDepth, animated: bool, frames: &[&[u8]], chunks: &[Vec<u8>]) -> Vec<u8> {
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
                let source = source(depth, animated, &frames, &chunks);
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
