#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

mod bindings;

pub use bindings::*;

pub const UPSTREAM_LIBAVIF_VERSION: &str = env!("JC_LIBAVIF_SYS_UPSTREAM_LIBAVIF_VERSION");
pub const UPSTREAM_LIBAOM_VERSION: &str = env!("JC_LIBAVIF_SYS_UPSTREAM_LIBAOM_VERSION");
