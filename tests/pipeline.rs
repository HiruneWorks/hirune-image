mod support;
use application::images::{
    AnimationPolicy, Encoding, JobInput, Limits, OutputRecipe, Recipe, sha256,
};
#[test]
fn multiple_outputs_share_one_measured_source_decode() {
    for source in [
        support::png(19, 11),
        support::fixture("finite1.gif"),
        support::fixture("p3-orientation6-gps.jpg"),
        support::fixture("rust-disposal-lossless.webp"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("source"), &source).unwrap();
        let job = JobInput {
            recipe: Recipe {
                animation_policy: AnimationPolicy::Preserve,
                outputs: [4, 8, 16, 32]
                    .into_iter()
                    .map(|size| OutputRecipe {
                        id: format!("size_{size}"),
                        max_width: size,
                        max_height: size,
                        encoding: Encoding::Lossless { effort: 0 },
                    })
                    .collect(),
            },
            limits: Limits::default(),
            source_sha256: sha256(&source),
            recipe_sha256: sha256(b"private pipeline test"),
        };
        let manifest = image_codec::process_job(directory.path(), &job).unwrap();
        assert_eq!(manifest.outputs.len(), 4);
        let metrics: serde_json::Value =
            serde_json::from_slice(&std::fs::read(directory.path().join("metrics.json")).unwrap())
                .unwrap();
        assert_eq!(metrics["source_decoder_starts"], 1);
        assert_eq!(metrics["decoded_frames"], manifest.source.image.frame_count);
    }
}
