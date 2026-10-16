use super::{Encoding, Limits, Recipe};
use crate::error::{ProcessingError, Result};
use std::collections::HashSet;
pub fn validate_recipe(recipe: &Recipe, limits: &Limits) -> Result<()> {
    let mut ids = HashSet::new();
    if recipe.outputs.is_empty() || recipe.outputs.len() > limits.outputs {
        return Err(ProcessingError::InvalidRecipe);
    }
    for o in &recipe.outputs {
        let id = o.id.as_bytes();
        if id.is_empty()
            || id.len() > 32
            || !id[0].is_ascii_lowercase()
            || !id
                .iter()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'_')
            || !ids.insert(&o.id)
            || o.max_width == 0
            || o.max_height == 0
            || o.max_width > limits.output_dimension
            || o.max_height > limits.output_dimension
            || o.encoding.effort() > 6
            || matches!(o.encoding,Encoding::Lossy{quality,..} if quality>100)
        {
            return Err(ProcessingError::InvalidRecipe);
        }
    }
    Ok(())
}
pub fn inside_dimensions(w: u32, h: u32, mw: u32, mh: u32) -> (u32, u32) {
    if w <= mw && h <= mh {
        return (w, h);
    }
    if u64::from(mw) * u64::from(h) <= u64::from(mh) * u64::from(w) {
        (
            mw,
            ((u64::from(h) * u64::from(mw) / u64::from(w)) as u32).max(1),
        )
    } else {
        (
            ((u64::from(w) * u64::from(mh) / u64::from(h)) as u32).max(1),
            mh,
        )
    }
}
