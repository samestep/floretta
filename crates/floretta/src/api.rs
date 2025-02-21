use crate::{
    ErrorImpl, NoValidate, Validate,
    forward::{self, ForwardTransform},
    reverse::{self, ReverseTransform},
};

/// An error that occurred during code transformation.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error {
    inner: ErrorImpl,
}

/// WebAssembly code transformation to perform forward mode automatic differentiation.
///
/// Use [`Forward::new`] to create an empty config, then use [`Forward::transform`] to process a
/// Wasm module.
///
/// For example, if you have [`wat`][] and [Wasmtime][] installed:
///
/// ```
/// use wasmtime::{Engine, Instance, Module, Store};
///
/// let input = wat::parse_str(r#"
/// (module
///   (func (export "square") (param f64) (result f64)
///     (f64.mul (local.get 0) (local.get 0))))
/// "#).unwrap();
///
/// let ad = floretta::Forward::new();
/// let output = ad.transform(&input).unwrap();
///
/// let engine = Engine::default();
/// let mut store = Store::new(&engine, ());
/// let module = Module::new(&engine, &output).unwrap();
/// let instance = Instance::new(&mut store, &module, &[]).unwrap();
/// let square = instance.get_typed_func::<(f64, f64), (f64, f64)>(&mut store, "square").unwrap();
///
/// assert_eq!(square.call(&mut store, (3., 1.)).unwrap(), (9., 6.));
/// ```
///
/// [`wat`]: https://crates.io/crates/wat
/// [wasmtime]: https://crates.io/crates/wasmtime
pub struct Forward {
    runner: Box<dyn ForwardTransform>,
    config: forward::Config,
}

impl Default for Forward {
    fn default() -> Self {
        Self {
            runner: Box::new(Validate),
            config: Default::default(),
        }
    }
}

impl Forward {
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

    /// Transform a WebAssembly module using this configuration.
    pub fn transform(&self, wasm: &[u8]) -> Result<Vec<u8>, Error> {
        self.runner
            .transform(&self.config, wasm)
            .map_err(|inner| Error { inner })
    }
}

/// WebAssembly code transformation to perform reverse mode automatic differentiation.
///
/// Create an empty config via [`Reverse::new`], use [`Reverse::export`] to specify one or more
/// functions to export the backward pass, and then use [`Reverse::transform`] to process a Wasm
/// module.
///
/// For example, if you have [`wat`][] and [Wasmtime][] installed:
///
/// ```
/// use wasmtime::{Engine, Instance, Module, Store};
///
/// let input = wat::parse_str(r#"
/// (module
///   (func (export "square") (param f64) (result f64)
///     (f64.mul (local.get 0) (local.get 0))))
/// "#).unwrap();
///
/// let mut ad = floretta::Reverse::new();
/// ad.export("square", "backprop");
/// let output = ad.transform(&input).unwrap();
///
/// let engine = Engine::default();
/// let mut store = Store::new(&engine, ());
/// let module = Module::new(&engine, &output).unwrap();
/// let instance = Instance::new(&mut store, &module, &[]).unwrap();
/// let square = instance.get_typed_func::<f64, f64>(&mut store, "square").unwrap();
/// let backprop = instance.get_typed_func::<f64, f64>(&mut store, "backprop").unwrap();
///
/// assert_eq!(square.call(&mut store, 3.).unwrap(), 9.);
/// assert_eq!(backprop.call(&mut store, 1.).unwrap(), 6.);
/// ```
///
/// [`wat`]: https://crates.io/crates/wat
/// [wasmtime]: https://crates.io/crates/wasmtime
pub struct Reverse {
    runner: Box<dyn ReverseTransform>,
    config: reverse::Config,
}

impl Default for Reverse {
    fn default() -> Self {
        Self {
            runner: Box::new(Validate),
            config: Default::default(),
        }
    }
}

impl Reverse {
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
    pub fn export(&mut self, forward: impl Into<String>, backward: impl Into<String>) {
        self.config.exports.insert(forward.into(), backward.into());
    }

    /// Transform a WebAssembly module using this configuration.
    pub fn transform(&self, wasm: &[u8]) -> Result<Vec<u8>, Error> {
        self.runner
            .transform(&self.config, wasm)
            .map_err(|inner| Error { inner })
    }
}
