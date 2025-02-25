//! [Automatic differentiation][] for [WebAssembly][]. See the [GitHub][] for more information.
//!
//! Here are some usage examples, assuming you have [`wat`][] and [Wasmtime][] installed.
//!
//! ## Forward mode
//!
//! Use [`Autodiff::new`] to create an empty config, then use [`Autodiff::forward`] to transform a
//! Wasm module to compute derivatives in forward mode.
//!
//! ```rust
//! use floretta::Autodiff;
//! use wasmtime::{Engine, Instance, Module, Store};
//!
//! let input = wat::parse_str(r#"
//! (module
//!   (func (export "square") (param f64) (result f64)
//!     (f64.mul (local.get 0) (local.get 0))))
//! "#).unwrap();
//!
//! let output = Autodiff::new().forward(&input).unwrap();
//!
//! let engine = Engine::default();
//! let mut store = Store::new(&engine, ());
//! let module = Module::new(&engine, &output).unwrap();
//! let instance = Instance::new(&mut store, &module, &[]).unwrap();
//! let square = instance.get_typed_func::<(f64, f64), (f64, f64)>(&mut store, "square").unwrap();
//!
//! assert_eq!(square.call(&mut store, (3., 1.)).unwrap(), (9., 6.));
//! ```
//!
//! ## Reverse mode
//!
//! Create an empty config via [`Autodiff::new`], use [`Autodiff::export`] to specify one or more
//! functions to export the backward pass, and then use [`Autodiff::reverse`] to transform a Wasm
//! module to compute derivatives in reverse mode.
//!
//! ```rust
//! use floretta::Autodiff;
//! use wasmtime::{Engine, Instance, Module, Store};
//!
//! let input = wat::parse_str(r#"
//! (module
//!   (func (export "square") (param f64) (result f64)
//!     (f64.mul (local.get 0) (local.get 0))))
//! "#).unwrap();
//!
//! let mut ad = Autodiff::new();
//! ad.export("square", "backprop");
//! let output = ad.reverse(&input).unwrap();
//!
//! let engine = Engine::default();
//! let mut store = Store::new(&engine, ());
//! let module = Module::new(&engine, &output).unwrap();
//! let instance = Instance::new(&mut store, &module, &[]).unwrap();
//! let square = instance.get_typed_func::<f64, f64>(&mut store, "square").unwrap();
//! let backprop = instance.get_typed_func::<f64, f64>(&mut store, "backprop").unwrap();
//!
//! assert_eq!(square.call(&mut store, 3.).unwrap(), 9.);
//! assert_eq!(backprop.call(&mut store, 1.).unwrap(), 6.);
//! ```
//!
//! [`wat`]: https://crates.io/crates/wat
//! [automatic differentiation]: https://en.wikipedia.org/wiki/Automatic_differentiation
//! [github]: https://github.com/samestep/floretta
//! [wasmtime]: https://crates.io/crates/wasmtime
//! [webassembly]: https://webassembly.org/

mod api;
mod forward;
mod helper;
mod reverse;
mod util;
mod validate;

#[cfg(feature = "names")]
mod name;

use wasm_encoder::reencode;
use wasmparser::{BinaryReaderError, Validator, WasmFeatures};

pub use api::*;

#[derive(Debug, thiserror::Error)]
enum ErrorImpl {
    #[error("Wasm parsing or validation error: {0}")]
    Parse(#[from] BinaryReaderError),

    #[error("code transformation error: {0}")]
    Transform(&'static str),

    #[error("Wasm reencoding error: {0}")]
    Reencode(#[from] reencode::Error),
}

type Result<T> = std::result::Result<T, ErrorImpl>;

trait Transform {
    fn forward(&self, config: &Autodiff, wasm_module: &[u8]) -> Result<Vec<u8>>;

    fn reverse(&self, config: &Autodiff, wasm_module: &[u8]) -> Result<Vec<u8>>;
}

// We make `Transform` a `trait` instead of just an `enum`, to facilitate dead code elimination when
// validation is not needed.

struct Validate;

struct NoValidate;

impl Transform for Validate {
    fn forward(&self, config: &Autodiff, wasm_module: &[u8]) -> Result<Vec<u8>> {
        let features = WasmFeatures::empty() | WasmFeatures::FLOATS;
        let validator = Validator::new_with_features(features);
        forward::transform(validator, config, wasm_module)
    }

    fn reverse(&self, config: &Autodiff, wasm_module: &[u8]) -> Result<Vec<u8>> {
        let features = WasmFeatures::empty() | WasmFeatures::MULTI_VALUE | WasmFeatures::FLOATS;
        let validator = Validator::new_with_features(features);
        reverse::transform(validator, config, wasm_module)
    }
}

impl Transform for NoValidate {
    fn forward(&self, config: &Autodiff, wasm_module: &[u8]) -> Result<Vec<u8>> {
        forward::transform((), config, wasm_module)
    }

    fn reverse(&self, config: &Autodiff, wasm_module: &[u8]) -> Result<Vec<u8>> {
        reverse::transform((), config, wasm_module)
    }
}
