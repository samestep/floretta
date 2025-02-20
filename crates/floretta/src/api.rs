use crate::{
    transform::{Config, NoValidate, Runner, Validate},
    ErrorImpl,
};

/// An error that occurred during code transformation.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error {
    inner: ErrorImpl,
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
        self.runner
            .transform(&self.config, wasm_module)
            .map_err(|inner| Error { inner })
    }
}
