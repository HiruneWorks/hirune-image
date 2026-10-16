use application::{
    error::ProcessingError as Error,
    images::{Budget, ImageFacts, Limits, inside_dimensions},
};
#[test]
fn every_budget_applies_at_its_boundary_and_checked_arithmetic_rejects_overflow() {
    let base = ImageFacts {
        width: 4,
        height: 4,
        frame_count: 2,
        duration_ms: 20,
        loop_count: 0,
    };
    for limits in [
        Limits {
            canvas_pixels: 16,
            ..Limits::default()
        },
        Limits {
            total_frame_pixels: 32,
            ..Limits::default()
        },
        Limits {
            decoded_bytes: 128,
            ..Limits::default()
        },
        Limits {
            spool_bytes: 128,
            ..Limits::default()
        },
        Limits {
            working_bytes: 320,
            ..Limits::default()
        },
        Limits {
            frames: 2,
            ..Limits::default()
        },
        Limits {
            duration_ms: 20,
            ..Limits::default()
        },
    ] {
        assert!(limits.preflight(&base).is_ok());
    }
    for limits in [
        Limits {
            canvas_pixels: 15,
            ..Limits::default()
        },
        Limits {
            total_frame_pixels: 31,
            ..Limits::default()
        },
        Limits {
            decoded_bytes: 127,
            ..Limits::default()
        },
        Limits {
            spool_bytes: 127,
            ..Limits::default()
        },
        Limits {
            working_bytes: 319,
            ..Limits::default()
        },
        Limits {
            frames: 1,
            ..Limits::default()
        },
        Limits {
            duration_ms: 19,
            ..Limits::default()
        },
    ] {
        assert_eq!(limits.preflight(&base), Err(Error::ImageLimitsExceeded));
    }
    let mut budget = Budget {
        frames: 0,
        pixels: u64::MAX,
        bytes: 0,
        duration_ms: 0,
    };
    assert_eq!(
        budget.frame(1, 1, 0, &Limits::default()),
        Err(Error::ImageLimitsExceeded)
    );
    assert_eq!(inside_dimensions(100, 50, 40, 40), (40, 20));
    assert_eq!(inside_dimensions(50, 100, 40, 40), (20, 40));
    assert_eq!(inside_dimensions(1, 8192, 4096, 4096), (1, 4096));
    assert_eq!(inside_dimensions(40, 20, 100, 100), (40, 20));
}

#[test]
fn inside_geometry_invariants_cover_extreme_aspect_ratios_and_u32_products() {
    let mut seed = 0x5eedu32;
    for _ in 0..10000 {
        let mut next = || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            seed.max(1)
        };
        let (w, h, mw, mh) = (next(), next(), next(), next());
        let (rw, rh) = inside_dimensions(w, h, mw, mh);
        assert!(rw >= 1 && rh >= 1 && rw <= w && rh <= h && rw <= mw && rh <= mh);
        assert_eq!(inside_dimensions(rw, rh, mw, mh), (rw, rh));
        let aspect_error = (u64::from(rw) * u64::from(h)).abs_diff(u64::from(rh) * u64::from(w));
        assert!(aspect_error <= u64::from(w.max(h)));
    }
}
