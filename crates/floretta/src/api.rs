use std::collections::{HashMap, hash_map::Entry};

use crate::{ErrorImpl, NoValidate, Transform, Validate};

/// An error that occurred during code transformation.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error {
    inner: ErrorImpl,
}

/// WebAssembly code transformations for automatic differentiation.
pub struct Autodiff {
    /// Name is a bit of a misnomer; this is just dynamic dispatch to choose whether or not to
    /// validate at the very beginning, so when doing the actual code transformation, validation
    /// dispatch is static.
    transform: Box<dyn Transform>,

    /// Exported functions whose backward passes should also be exported.
    pub(crate) exports: HashMap<String, String>,

    /// Whether to include the names section in the output Wasm.
    #[cfg(feature = "names")]
    pub(crate) names: bool,
}

impl Default for Autodiff {
    fn default() -> Self {
        Self::new()
    }
}

impl Autodiff {
    /// Default configuration.
    pub fn new() -> Self {
        Self {
            transform: Box::new(Validate),

            exports: HashMap::new(),

            #[cfg(feature = "names")]
            names: false,
        }
    }

    /// Do not validate input Wasm.
    pub fn no_validate() -> Self {
        Self {
            transform: Box::new(NoValidate),

            exports: HashMap::new(),

            #[cfg(feature = "names")]
            names: false,
        }
    }

    /// Include the name section in the output Wasm.
    #[cfg(feature = "names")]
    pub fn names(&mut self) {
        self.names = true;
    }

    /// In the output Wasm, also export the derivative counterpart of an export from the input Wasm.
    pub fn export(&mut self, primal: impl Into<String>, derivative: impl Into<String>) {
        match self.exports.entry(primal.into()) {
            Entry::Occupied(entry) => panic!("mapping already exists for export {:?}", entry.key()),
            Entry::Vacant(entry) => {
                entry.insert(derivative.into());
            }
        }
    }

    /// Transform a WebAssembly module to compute derivatives in forward mode.
    pub fn forward(&self, wasm: &[u8]) -> Result<Vec<u8>, Error> {
        self.transform
            .forward(self, wasm)
            .map_err(|inner| Error { inner })
    }

    /// Transform a WebAssembly module to compute derivatives in reverse mode.
    pub fn reverse(&self, wasm: &[u8]) -> Result<Vec<u8>, Error> {
        self.transform
            .reverse(self, wasm)
            .map_err(|inner| Error { inner })
    }
}
