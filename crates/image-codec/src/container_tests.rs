use crate::container::{gif_structure, validate_jpeg, validate_png, webp_structure};
use application::images::Limits;

#[test]
fn bounded_header_mutations_and_truncations_do_not_panic() {
    let fixtures: [&[u8]; 4] = [
        include_bytes!("../../../tests/fixtures/finite1.gif"),
        include_bytes!("../../../tests/fixtures/rust-disposal-lossless.webp"),
        include_bytes!("../../../tests/fixtures/srgb-orientation6-gps.jpg"),
        include_bytes!("../../../tests/fixtures/p3-color.png"),
    ];
    let limits = Limits {
        frames: 8,
        duration_ms: 2000,
        ..Limits::default()
    };
    for (kind, fixture) in fixtures.into_iter().enumerate() {
        let inspect = |data: &[u8]| match kind {
            0 => gif_structure(data, &limits).map(|_| ()),
            1 => webp_structure(data, &limits).map(|_| ()),
            2 => validate_jpeg(data).map(|_| ()),
            _ => validate_png(data).map(|_| ()),
        };
        assert!(inspect(fixture).is_ok());
        for length in (0..fixture.len()).step_by((fixture.len() / 64).max(1)) {
            assert!(inspect(&fixture[..length]).is_err());
        }
        let mut seed = 0x5eedu32;
        for _ in 0..256 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let mut changed = fixture.to_vec();
            changed[seed as usize % fixture.len()] ^= 255;
            let _ = inspect(&changed);
        }
    }
}
