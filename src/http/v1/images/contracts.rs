use application::{
    error::{ProcessingError as Error, Result},
    images::{self, AnimationPolicy, Limits, validate_recipe},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecipeInput {
    #[schemars(range(min = 1, max = 1))]
    pub contract_version: u32,
    pub animation_policy: AnimationPolicy,
    #[schemars(length(min = 1))]
    pub outputs: Vec<OutputInput>,
}
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OutputInput {
    #[schemars(length(min = 1, max = 32), regex(pattern = "^[a-z][a-z0-9_]{0,31}$"))]
    pub id: String,
    pub resize: ResizeInput,
    pub encoding: EncodingInput,
}
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResizeInput {
    pub fit: ResizeFit,
    #[schemars(range(min = 1, max = 16383))]
    pub max_width: u32,
    #[schemars(range(min = 1, max = 16383))]
    pub max_height: u32,
}
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResizeFit {
    Inside,
}
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    Webp,
}
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum EncodingInput {
    Lossy {
        format: OutputFormat,
        #[schemars(range(min = 0, max = 100))]
        quality: u8,
        #[schemars(range(min = 0, max = 6))]
        effort: u8,
    },
    Lossless {
        format: OutputFormat,
        #[schemars(range(min = 0, max = 6))]
        effort: u8,
    },
}
impl RecipeInput {
    pub fn into_recipe(self, limits: &Limits) -> Result<images::Recipe> {
        if self.contract_version != 1 {
            return Err(Error::InvalidRecipe);
        }
        let recipe = images::Recipe {
            animation_policy: self.animation_policy,
            outputs: self
                .outputs
                .into_iter()
                .map(|o| images::OutputRecipe {
                    id: o.id,
                    max_width: o.resize.max_width,
                    max_height: o.resize.max_height,
                    encoding: match o.encoding {
                        EncodingInput::Lossy {
                            quality, effort, ..
                        } => images::Encoding::Lossy { quality, effort },
                        EncodingInput::Lossless { effort, .. } => {
                            images::Encoding::Lossless { effort }
                        }
                    },
                })
                .collect(),
        };
        validate_recipe(&recipe, limits)?;
        Ok(recipe)
    }
}
