mod support;
use application::images::Limits;
use support::*;

#[tokio::test]
async fn raised_limits_reach_http_codec_schemas_and_independent_verifier() {
    let limits = Limits {
        outputs: 1024,
        recipe_bytes: 256 << 10,
        output_dimension: 8192,
        frames: 121,
        duration_ms: 121_000,
        output_bytes: 128 << 20,
        working_bytes: 1 << 30,
        child_memory_bytes: 2 << 30,
        concurrency: 5,
        ..Limits::default()
    };
    let server = Server::start(limits.clone()).await;
    let capabilities: serde_json::Value = server
        .client
        .get(format!("{}/v1/capabilities", server.url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        capabilities["limits"],
        serde_json::to_value(&limits).unwrap()
    );
    assert_eq!(capabilities["codec_limits"]["webp_dimension"], 16383);
    let mut animation = Vec::new();
    {
        let mut encoder = gif::Encoder::new(&mut animation, 1, 1, &[255, 0, 0]).unwrap();
        encoder.set_repeat(gif::Repeat::Finite(2)).unwrap();
        for _ in 0..121 {
            let frame = gif::Frame {
                width: 1,
                height: 1,
                delay: 100,
                buffer: std::borrow::Cow::Borrowed(&[0]),
                ..Default::default()
            };
            encoder.write_frame(&frame).unwrap();
        }
    }
    let documents = hirune_image::contracts::documents();
    for (source, sizes) in [
        (png(8192, 1), vec![(8192, 1); 5]),
        (animation, vec![(1, 1)]),
        (png(1, 1), vec![(1, 1); 1024]),
    ] {
        let mut recipe: serde_json::Value =
            serde_json::from_str(&recipe("lossless", "preserve", &sizes)).unwrap();
        for (i, output) in recipe["outputs"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .enumerate()
        {
            output["id"] = serde_json::json!(format!("variant_{i:024}"));
        }
        let recipe = recipe.to_string();
        let recipe_schema = &documents
            .iter()
            .find(|(p, _)| p.ends_with("recipe-v1.schema.json"))
            .unwrap()
            .1;
        assert!(
            jsonschema::validator_for(recipe_schema)
                .unwrap()
                .is_valid(&serde_json::from_str::<serde_json::Value>(&recipe).unwrap())
        );
        let response = server.process(source.clone(), &recipe).await;
        assert_eq!(
            response.status(),
            200,
            "{}",
            response.text().await.unwrap_or_default()
        );
        let payload = response.bytes().await.unwrap();
        let directory = tempfile::tempdir().unwrap();
        for (name, bytes) in [
            ("response", payload.as_ref()),
            ("source", source.as_slice()),
            ("recipe", recipe.as_bytes()),
        ] {
            std::fs::write(directory.path().join(name), bytes).unwrap();
        }
        std::fs::write(
            directory.path().join("limits"),
            serde_json::to_vec(&capabilities).unwrap(),
        )
        .unwrap();
        let python = if cfg!(windows) { "python" } else { "python3" };
        assert!(
            std::process::Command::new(python)
                .arg("scripts/verify_response.py")
                .arg(directory.path().join("response"))
                .arg("--source")
                .arg(directory.path().join("source"))
                .arg("--recipe")
                .arg(directory.path().join("recipe"))
                .arg("--limits")
                .arg(directory.path().join("limits"))
                .status()
                .unwrap()
                .success()
        );
        let boundary = payload.split(|&b| b == b'\n').next().unwrap();
        let boundary = std::str::from_utf8(&boundary[2..boundary.len() - 1]).unwrap();
        let mut multipart = multer::Multipart::new(
            futures_util::stream::once(async { Ok::<_, std::io::Error>(payload.clone()) }),
            boundary,
        );
        let manifest: serde_json::Value = serde_json::from_slice(
            &multipart
                .next_field()
                .await
                .unwrap()
                .unwrap()
                .bytes()
                .await
                .unwrap(),
        )
        .unwrap();
        let schema = &documents
            .iter()
            .find(|(p, _)| p.ends_with("manifest-v1.schema.json"))
            .unwrap()
            .1;
        assert!(
            jsonschema::validator_for(schema)
                .unwrap()
                .is_valid(&manifest)
        );
        assert_eq!(manifest["outputs"].as_array().unwrap().len(), sizes.len());
        if manifest["source"]["frame_count"] == 121 {
            assert_eq!(manifest["outputs"][0]["duration_ms"], 121000);
            assert_eq!(manifest["outputs"][0]["frame_count"], 121);
        }
        server.released().await;
    }
    let rejected = recipe("lossless", "preserve", &[(8193, 1)]);
    assert_eq!(server.process(png(1, 1), &rejected).await.status(), 400);
    server.released().await;
}

#[test]
fn operator_budgets_can_increase_but_overflow_and_codec_caps_cannot() {
    let mut limits = Limits::default();
    limits.source_bytes *= 2;
    limits.request_bytes *= 3;
    limits.recipe_bytes *= 2;
    limits.input_dimension *= 2;
    limits.canvas_pixels *= 2;
    limits.frames *= 2;
    limits.total_frame_pixels *= 2;
    limits.decoded_bytes *= 2;
    limits.working_bytes *= 2;
    limits.spool_bytes *= 2;
    limits.duration_ms *= 2;
    limits.outputs *= 2;
    limits.output_bytes *= 2;
    limits.output_dimension = 16383;
    limits.concurrency = 5;
    limits.deadline_ms *= 2;
    limits.idle_ms *= 2;
    limits.child_memory_bytes *= 2;
    limits.child_cpu_seconds *= 2;
    assert!(limits.validate().is_ok());
    for invalid in [
        Limits {
            output_dimension: 16384,
            ..limits.clone()
        },
        Limits {
            request_bytes: u64::MAX,
            ..limits.clone()
        },
        Limits {
            outputs: usize::MAX,
            ..limits.clone()
        },
        Limits {
            child_cpu_seconds: u64::MAX,
            ..limits.clone()
        },
        Limits {
            duration_ms: u32::MAX,
            ..limits.clone()
        },
        Limits {
            concurrency: usize::MAX,
            ..limits
        },
    ] {
        assert!(invalid.validate().is_err());
    }
}
