mod support;
use application::images::Limits;
use support::*;

#[tokio::test]
async fn rgb_icc_pixels_match_independent_littlecms_and_alpha_is_exact() {
    let server = Server::start(Limits::default()).await;
    let source = fixture("p3-color.png");
    let recipe = recipe("lossless", "preserve", &[(8, 4)]);
    let (_, outputs) = verify(
        server.process(source.clone(), &recipe).await,
        &source,
        &recipe,
    )
    .await;
    let expected = fixture("p3-color-expected.rgba");
    image_codec::decode_frames(&outputs[0], &Limits::default(), |actual, _| {
        assert_eq!(actual.len(), expected.len());
        for (a, b) in actual
            .as_chunks::<4>()
            .0
            .iter()
            .zip(expected.as_chunks::<4>().0.iter())
        {
            assert_eq!(a[3], b[3]);
            if b[3] > 0 {
                for channel in 0..3 {
                    assert!(
                        a[channel].abs_diff(b[channel]) <= 2,
                        "RGB mismatch: {a:?} vs {b:?}"
                    );
                }
            }
        }
        Ok(())
    })
    .unwrap();
    server.released().await;
}

#[tokio::test]
async fn identical_displays_keep_every_delay_and_finite_loop() {
    let mut source = Vec::new();
    {
        let mut encoder = gif::Encoder::new(&mut source, 1, 1, &[255, 0, 0]).unwrap();
        encoder.set_repeat(gif::Repeat::Finite(2)).unwrap();
        for delay in [10, 20, 30] {
            encoder
                .write_frame(&gif::Frame {
                    width: 1,
                    height: 1,
                    delay,
                    buffer: std::borrow::Cow::Borrowed(&[0]),
                    ..Default::default()
                })
                .unwrap();
        }
    }
    let server = Server::start(Limits::default()).await;
    let recipe = recipe("lossless", "preserve", &[(1, 1)]);
    let (manifest, outputs) = verify(
        server.process(source.clone(), &recipe).await,
        &source,
        &recipe,
    )
    .await;
    assert_eq!(manifest.outputs[0].image.loop_count, 3);
    let mut delays = Vec::new();
    image_codec::decode_frames(&outputs[0], &Limits::default(), |pixels, delay| {
        assert_eq!(pixels, [255, 0, 0, 255]);
        delays.push(delay);
        Ok(())
    })
    .unwrap();
    assert_eq!(delays, [100, 200, 300]);
    server.released().await;
}

#[tokio::test]
async fn resized_disposal_animation_matches_independent_linear_area_reference() {
    let server = Server::start(Limits::default()).await;
    let source = fixture("disposal.gif");
    let recipe = recipe("lossless", "preserve", &[(4, 3)]);
    let (_, outputs) = verify(
        server.process(source.clone(), &recipe).await,
        &source,
        &recipe,
    )
    .await;
    let mut index = 0;
    image_codec::decode_frames(&outputs[0], &Limits::default(), |actual, delay| {
        let expected = fixture(&format!("disposal-small-expected-{index}.rgba"));
        assert_eq!(actual.len(), expected.len());
        assert_eq!(delay, [100, 700, 300, 500][index]);
        for (a, b) in actual
            .as_chunks::<4>()
            .0
            .iter()
            .zip(expected.as_chunks::<4>().0.iter())
        {
            assert_eq!(a[3], b[3]);
            if b[3] > 0 {
                for channel in 0..3 {
                    assert!(
                        a[channel].abs_diff(b[channel]) <= 1,
                        "resized RGBA mismatch: {a:?} vs {b:?}"
                    );
                }
            }
        }
        index += 1;
        Ok(())
    })
    .unwrap();
    assert_eq!(index, 4);
    server.released().await;
}
