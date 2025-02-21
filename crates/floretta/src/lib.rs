//! Automatic differentiation for WebAssembly.
//!
//! See [`Forward`] for forward mode or [`Reverse`] for reverse mode.

mod api;
mod forward;
mod helper;
#[cfg(feature = "names")]
mod name;
mod reverse;
mod util;
mod validate;

use wasm_encoder::reencode;
use wasmparser::BinaryReaderError;

pub use api::*;

#[derive(Debug, thiserror::Error)]
enum ErrorImpl {
    #[error("Wasm parsing or validation error: {0}")]
    Parse(#[from] BinaryReaderError),

    #[error("Wasm reencoding error: {0}")]
    Reencode(#[from] reencode::Error),
}

type Result<T> = std::result::Result<T, ErrorImpl>;

struct Validate;

struct NoValidate;
