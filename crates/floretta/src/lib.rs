//! Reverse-mode automatic differentiation for WebAssembly.
//!
//! The typical workflow is to create an empty config via [`Autodiff::new`], use
//! [`Autodiff::export`] to specify one or more functions to export the backward pass, and then use
//! [`Autodiff::transform`] to process a Wasm module.
//!
//! For example, if you have [`wat`][] and [Wasmtime][] installed:
//!
//! ```
//! use wasmtime::{Engine, Instance, Module, Store};
//!
//! let input = wat::parse_str(r#"
//! (module
//!   (func (export "square") (param f64) (result f64)
//!     (f64.mul (local.get 0) (local.get 0))))
//! "#).unwrap();
//!
//! let mut ad = floretta::Autodiff::new();
//! ad.export("square", "backprop");
//! let output = ad.transform(&input).unwrap();
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
//! [wasmtime]: https://crates.io/crates/wasmtime

mod helper;
#[cfg(feature = "names")]
mod name;
mod run;
mod util;
mod validate;

use std::collections::HashMap;

use wasm_encoder::reencode;
use wasmparser::{BinaryReaderError, Validator, WasmFeatures};

/// An error that occurred during code transformation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The error occurred while parsing or validating the input Wasm.
    #[error("Wasm parsing or validation error: {0}")]
    Parse(#[from] BinaryReaderError),

    /// The error occurred while reencoding part of the input Wasm into the output Wasm.
    #[error("Wasm reencoding error: {0}")]
    Reencode(#[from] reencode::Error),
}

#[derive(Default)]
struct Config {
    /// Exported functions whose backward passes should also be exported.
    exports: HashMap<String, String>,

    /// Whether to include the names section in the output Wasm.
    #[cfg(feature = "names")]
    names: bool,
}

/// WebAssembly code transformation to perform reverse-mode automatic differentiation.
pub struct Autodiff {
    runner: Box<dyn Runner>,
    config: Config,
}

impl Default for Autodiff {
    fn default() -> Self {
        Self {
            runner: Box::new(Validate),
            config: Default::default(),
        }
    }
}

impl Autodiff {
    /// Default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Do not validate input Wasm.
    pub fn no_validate() -> Self {
        Self {
            runner: Box::new(NoValidate),
            config: Default::default(),
        }
    }

    /// Include the name section in the output Wasm.
    #[cfg(feature = "names")]
    pub fn names(&mut self) {
        self.config.names = true;
    }

    /// Export the backward pass of a function that is already exported.
    pub fn export(&mut self, function: impl ToString, gradient: impl ToString) {
        self.config
            .exports
            .insert(function.to_string(), gradient.to_string());
    }

    /// Transform a WebAssembly module using this configuration.
    pub fn transform(&self, wasm_module: &[u8]) -> Result<Vec<u8>, Error> {
        self.runner.transform(&self.config, wasm_module)
    }
}

trait Runner {
    fn transform(&self, config: &Config, wasm_module: &[u8]) -> Result<Vec<u8>, Error>;
}

// We make `Runner` a `trait` instead of just an `enum`, to facilitate dead code elimination when
// validation is not needed.

struct Validate;

struct NoValidate;

impl Runner for Validate {
    fn transform(&self, config: &Config, wasm_module: &[u8]) -> Result<Vec<u8>, Error> {
        let features = WasmFeatures::empty() | WasmFeatures::FLOATS;
        let validator = Validator::new_with_features(features);
        run::transform(validator, config, wasm_module)
    }
}

impl Runner for NoValidate {
    fn transform(&self, config: &Config, wasm_module: &[u8]) -> Result<Vec<u8>, Error> {
        run::transform((), config, wasm_module)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use goldenfile::Mint;
    use wasmtime::{Engine, Instance, Module, Store};

    #[test]
    #[cfg(feature = "names")]
    fn test_names() {
        let input = wat::parse_str(include_str!("wat/names.wat")).unwrap();
        let mut ad = crate::Autodiff::new();
        ad.names();
        let output = wasmprinter::print_bytes(ad.transform(&input).unwrap()).unwrap();
        let mut mint = Mint::new("src/out");
        let mut file = mint.new_goldenfile("names.wat").unwrap();
        file.write_all(output.as_bytes()).unwrap();
    }

    #[test]
    fn test_square() {
        let input = wat::parse_str(include_str!("wat/square.wat")).unwrap();

        let mut ad = crate::Autodiff::new();
        ad.export("square", "backprop");
        let output = ad.transform(&input).unwrap();

        let engine = Engine::default();
        let mut store = Store::new(&engine, ());
        let module = Module::new(&engine, &output).unwrap();
        let instance = Instance::new(&mut store, &module, &[]).unwrap();
        let square = instance
            .get_typed_func::<f64, f64>(&mut store, "square")
            .unwrap();
        let backprop = instance
            .get_typed_func::<f64, f64>(&mut store, "backprop")
            .unwrap();

        assert_eq!(square.call(&mut store, 3.).unwrap(), 9.);
        assert_eq!(backprop.call(&mut store, 1.).unwrap(), 6.);
    }

    #[test]
    fn test_loop() {
        let input = wat::parse_str(include_str!("wat/loop.wat")).unwrap();

        let mut ad = crate::Autodiff::new();
        ad.export("loop", "backprop");
        let output = ad.transform(&input).unwrap();

        let engine = Engine::default();
        let mut store = Store::new(&engine, ());
        let module = Module::new(&engine, &output).unwrap();
        let instance = Instance::new(&mut store, &module, &[]).unwrap();
        let fwd = instance
            .get_typed_func::<f64, f64>(&mut store, "loop")
            .unwrap();
        let bwd = instance
            .get_typed_func::<f64, f64>(&mut store, "backprop")
            .unwrap();

        assert_eq!(fwd.call(&mut store, 1.1).unwrap(), -0.99);
        assert_eq!(bwd.call(&mut store, 1.).unwrap(), 0.20000000000000018);
    }

    #[test]
    fn test_f32_mul() {
        let input = wat::parse_str(include_str!("wat/f32_mul.wat")).unwrap();

        let mut ad = crate::Autodiff::new();
        ad.export("mul", "backprop");
        let output = ad.transform(&input).unwrap();

        let engine = Engine::default();
        let mut store = Store::new(&engine, ());
        let module = Module::new(&engine, &output).unwrap();
        let instance = Instance::new(&mut store, &module, &[]).unwrap();
        let fwd = instance
            .get_typed_func::<(f32, f32), f32>(&mut store, "mul")
            .unwrap();
        let bwd = instance
            .get_typed_func::<f32, (f32, f32)>(&mut store, "backprop")
            .unwrap();

        assert_eq!(fwd.call(&mut store, (3., 2.)).unwrap(), 6.);
        assert_eq!(bwd.call(&mut store, 1.).unwrap(), (2., 3.));
    }

    #[test]
    fn test_f32_div() {
        let input = wat::parse_str(include_str!("wat/f32_div.wat")).unwrap();

        let mut ad = crate::Autodiff::new();
        ad.export("div", "backprop");
        let output = ad.transform(&input).unwrap();

        let engine = Engine::default();
        let mut store = Store::new(&engine, ());
        let module = Module::new(&engine, &output).unwrap();
        let instance = Instance::new(&mut store, &module, &[]).unwrap();
        let fwd = instance
            .get_typed_func::<(f32, f32), f32>(&mut store, "div")
            .unwrap();
        let bwd = instance
            .get_typed_func::<f32, (f32, f32)>(&mut store, "backprop")
            .unwrap();

        assert_eq!(fwd.call(&mut store, (3., 2.)).unwrap(), 1.5);
        assert_eq!(bwd.call(&mut store, 1.).unwrap(), (0.5, -0.75));
    }

    #[test]
    fn test_f64_mul() {
        let input = wat::parse_str(include_str!("wat/f64_mul.wat")).unwrap();

        let mut ad = crate::Autodiff::new();
        ad.export("mul", "backprop");
        let output = ad.transform(&input).unwrap();

        let engine = Engine::default();
        let mut store = Store::new(&engine, ());
        let module = Module::new(&engine, &output).unwrap();
        let instance = Instance::new(&mut store, &module, &[]).unwrap();
        let fwd = instance
            .get_typed_func::<(f64, f64), f64>(&mut store, "mul")
            .unwrap();
        let bwd = instance
            .get_typed_func::<f64, (f64, f64)>(&mut store, "backprop")
            .unwrap();

        assert_eq!(fwd.call(&mut store, (3., 2.)).unwrap(), 6.);
        assert_eq!(bwd.call(&mut store, 1.).unwrap(), (2., 3.));
    }

    #[test]
    fn test_f64_div() {
        let input = wat::parse_str(include_str!("wat/f64_div.wat")).unwrap();

        let mut ad = crate::Autodiff::new();
        ad.export("div", "backprop");
        let output = ad.transform(&input).unwrap();

        let engine = Engine::default();
        let mut store = Store::new(&engine, ());
        let module = Module::new(&engine, &output).unwrap();
        let instance = Instance::new(&mut store, &module, &[]).unwrap();
        let fwd = instance
            .get_typed_func::<(f64, f64), f64>(&mut store, "div")
            .unwrap();
        let bwd = instance
            .get_typed_func::<f64, (f64, f64)>(&mut store, "backprop")
            .unwrap();

        assert_eq!(fwd.call(&mut store, (3., 2.)).unwrap(), 1.5);
        assert_eq!(bwd.call(&mut store, 1.).unwrap(), (0.5, -0.75));
    }
}
