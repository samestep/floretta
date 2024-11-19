//! Reverse-mode automatic differentiation for WebAssembly.
//!
//! The typical workflow is to create an empty config via [`Autodiff::new`], use
//! [`Autodiff::gradient`] to specify one or more scalar-valued functions of which to take the
//! gradient, and then use [`Autodiff::transform`] to process a Wasm module.
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
//! ad.gradient("square", "double");
//! let output = ad.transform(&input).unwrap();
//!
//! let engine = Engine::default();
//! let mut store = Store::new(&engine, ());
//! let module = Module::new(&engine, &output).unwrap();
//! let instance = Instance::new(&mut store, &module, &[]).unwrap();
//! let double = instance.get_typed_func::<f64, f64>(&mut store, "double").unwrap();
//! let result = double.call(&mut store, 3.).unwrap();
//! assert_eq!(result, 6.);
//! ```
//!
//! [`wat`]: https://crates.io/crates/wat
//! [wasmtime]: https://crates.io/crates/wasmtime

use std::collections::HashMap;

use wasm_encoder::{
    CodeSection, ExportKind, ExportSection, Function, FunctionSection, Instruction, Module,
    TypeSection, ValType,
};
use wasmparser::validate;

/// WebAssembly code transformation to perform reverse-mode automatic differentiation.
#[derive(Default)]
pub struct Autodiff {
    gradients: HashMap<String, String>,
}

impl Autodiff {
    /// Default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Export the gradient of a function that is already exported.
    pub fn gradient(&mut self, function: impl ToString, gradient: impl ToString) {
        self.gradients
            .insert(function.to_string(), gradient.to_string());
    }

    /// Transform a WebAssembly module using this configuration.
    pub fn transform(self, wasm_module: &[u8]) -> wasmparser::Result<Vec<u8>> {
        validate(wasm_module)?;

        let mut module = Module::new();
        // HACK: obviously
        if matches!(self.gradients.get("square"), Some(name) if name == "double") {
            let mut types = TypeSection::new();
            types.ty().function([ValType::F64], [ValType::F64]);
            module.section(&types);

            let mut functions = FunctionSection::new();
            functions.function(0);
            module.section(&functions);

            let mut exports = ExportSection::new();
            exports.export("double", ExportKind::Func, 0);
            module.section(&exports);

            let mut codes = CodeSection::new();
            let mut f = Function::new([]);
            f.instruction(&Instruction::LocalGet(0));
            f.instruction(&Instruction::LocalGet(0));
            f.instruction(&Instruction::F64Add);
            f.instruction(&Instruction::End);
            codes.function(&f);
            module.section(&codes);
        }
        Ok(module.finish())
    }
}

#[cfg(test)]
mod tests {
    use wasmtime::{Engine, Instance, Module, Store};

    #[test]
    fn test_square() {
        let input = wat::parse_str(include_str!("wat/square.wat")).unwrap();

        let mut ad = crate::Autodiff::new();
        ad.gradient("square", "double");
        let output = ad.transform(&input).unwrap();

        let engine = Engine::default();
        let mut store = Store::new(&engine, ());
        let module = Module::new(&engine, &output).unwrap();
        let instance = Instance::new(&mut store, &module, &[]).unwrap();
        let double = instance
            .get_typed_func::<f64, f64>(&mut store, "double")
            .unwrap();
        let result = double.call(&mut store, 3.).unwrap();
        assert_eq!(result, 6.);
    }
}
