use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnimationPolicy {
    Preserve,
    FirstFrame,
    Reject,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceFormat {
    Jpeg,
    Png,
    Gif,
    Webp,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub enum MediaType {
    #[serde(rename = "image/webp")]
    Webp,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    pub animation_policy: AnimationPolicy,
    pub outputs: Vec<OutputRecipe>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OutputRecipe {
    pub id: String,
    pub max_width: u32,
    pub max_height: u32,
    pub encoding: Encoding,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum Encoding {
    Lossy { quality: u8, effort: u8 },
    Lossless { effort: u8 },
}
impl Encoding {
    pub fn effort(&self) -> u8 {
        match *self {
            Self::Lossy { effort, .. } | Self::Lossless { effort } => effort,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ImageFacts {
    #[schemars(range(min = 1))]
    pub width: u32,
    #[schemars(range(min = 1))]
    pub height: u32,
    #[schemars(range(min = 1))]
    pub frame_count: u32,
    #[schemars(range(min = 0, max = 2147483647))]
    pub duration_ms: u32,
    pub loop_count: u16,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct SourceFacts {
    pub format: SourceFormat,
    pub byte_size: u64,
    pub orientation: u8,
    #[serde(flatten)]
    pub image: ImageFacts,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct OutputFacts {
    #[schemars(regex(pattern = "^[a-z][a-z0-9_]{0,31}$"))]
    pub id: String,
    pub media_type: MediaType,
    #[schemars(range(min = 12, max = 4294967294u64))]
    pub byte_size: u64,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub sha256: String,
    #[serde(flatten)]
    pub image: ImageFacts,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProcessorInfo {
    pub version: String,
    pub build_id: String,
    pub pipeline_version: String,
    pub image: String,
    pub gif: String,
    pub libwebp_encoder: String,
    pub libwebp_decoder: String,
    pub libwebp_demux: String,
    pub libwebp_mux: String,
    pub color_engine: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    #[schemars(range(min = 1, max = 1))]
    pub contract_version: u32,
    pub processor: ProcessorInfo,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub source_sha256: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub recipe_sha256: String,
    pub source: SourceFacts,
    #[schemars(length(min = 1))]
    pub outputs: Vec<OutputFacts>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JobInput {
    pub recipe: Recipe,
    pub limits: super::Limits,
    pub source_sha256: String,
    pub recipe_sha256: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum JobResult {
    Success {
        manifest: Box<Manifest>,
    },
    Failure {
        error: crate::error::ProcessingError,
    },
}
pub fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
