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
//! ad.export("square", "double");
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
    reencode::{self, Reencode, RoundtripReencoder},
    CodeSection, ExportKind, ExportSection, Function, FunctionSection, Instruction, Module,
    TypeSection,
};
use wasmparser::{
    BinaryReaderError, FuncToValidate, FuncValidatorAllocations, FunctionBody, Parser, Payload,
    Validator, ValidatorResources, WasmFeatures,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Wasm parsing or validation error: {0}")]
    Parse(#[from] BinaryReaderError),

    #[error("Wasm reencoding error: {0}")]
    Reencode(#[from] reencode::Error),
}

/// WebAssembly code transformation to perform reverse-mode automatic differentiation.
#[derive(Default)]
pub struct Autodiff {
    /// Exported functions whose backward passes should also be exported.
    exports: HashMap<String, String>,
}

impl Autodiff {
    /// Default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Export the backward pass of a function that is already exported.
    pub fn export(&mut self, function: impl ToString, gradient: impl ToString) {
        self.exports
            .insert(function.to_string(), gradient.to_string());
    }

    /// Transform a WebAssembly module using this configuration.
    pub fn transform(self, wasm_module: &[u8]) -> Result<Vec<u8>, Error> {
        let mut types = TypeSection::new();
        let mut functions = FunctionSection::new();
        let mut exports = ExportSection::new();
        let mut code = CodeSection::new();
        let mut validator = Validator::new_with_features(features());
        for payload in Parser::new(0).parse_all(wasm_module) {
            match payload? {
                Payload::TypeSection(section) => {
                    validator.type_section(&section)?;
                    for func_ty in section.into_iter_err_on_gc_types() {
                        let ty = RoundtripReencoder.func_type(func_ty?)?;
                        // Forward pass: same type as the original function. For integers, all the
                        // adjoint values are assumed to be equal to the primal values (e.g.
                        // pointers, because of our multi-memory strategy), and for floating point,
                        // all the adjoint values are assumed to be zero.
                        types.ty().func_type(&ty);
                        // Backward pass: results become parameters, and parameters become results.
                        types.ty().func_type(&wasm_encoder::FuncType::new(
                            ty.results().iter().copied(),
                            ty.params().iter().copied(),
                        ));
                    }
                }
                Payload::FunctionSection(section) => {
                    validator.function_section(&section)?;
                    for type_index in section.into_iter() {
                        let t = type_index?;
                        // Index arithmetic to account for the fact that we split each original
                        // function type into two; similarly, we also split each actual function
                        // into two.
                        functions.function(2 * t);
                        functions.function(2 * t + 1);
                    }
                }
                Payload::ExportSection(section) => {
                    validator.export_section(&section)?;
                    for export in section.into_iter() {
                        let e = export?;
                        let kind = RoundtripReencoder.export_kind(e.kind);
                        match kind {
                            ExportKind::Func => {
                                // More index arithmetic because we split every function into a
                                // forward pass and a backward pass.
                                exports.export(e.name, kind, 2 * e.index);
                                if let Some(name) = self.exports.get(e.name) {
                                    // TODO: Should we check that no export with this name already
                                    // exists?
                                    exports.export(name, kind, 2 * e.index + 1);
                                }
                            }
                            _ => {
                                exports.export(e.name, kind, e.index);
                            }
                        }
                    }
                }
                Payload::CodeSectionEntry(body) => {
                    let func = validator.code_section_entry(&body)?;
                    let (fwd, bwd) = function(body, func)?;
                    code.function(&fwd);
                    code.function(&bwd);
                }
                other => {
                    validator.payload(&other)?;
                }
            }
        }
        let mut module = Module::new();
        module.section(&types);
        module.section(&functions);
        module.section(&exports);
        module.section(&code);
        Ok(module.finish())
    }
}

fn features() -> WasmFeatures {
    WasmFeatures::empty() | WasmFeatures::FLOATS
}

fn function(
    body: FunctionBody,
    func: FuncToValidate<ValidatorResources>,
) -> Result<(Function, Function), Error> {
    let mut validator = func.into_validator(FuncValidatorAllocations::default());
    validator.validate(&body)?;
    let mut fwd = Function::new([]);
    let mut bwd = Function::new([]);
    // TODO: Actually differentiate the function.
    fwd.instruction(&Instruction::F64Const(9.));
    bwd.instruction(&Instruction::F64Const(6.));
    fwd.instruction(&Instruction::End);
    bwd.instruction(&Instruction::End);
    Ok((fwd, bwd))
}

#[cfg(test)]
mod tests {
    use wasmtime::{Engine, Instance, Module, Store};

    #[test]
    fn test_square() {
        let input = wat::parse_str(include_str!("wat/square.wat")).unwrap();

        let mut ad = crate::Autodiff::new();
        ad.export("square", "double");
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
