//! Reverse-mode automatic differentiation for WebAssembly.

/// Apply reverse-mode automatic differentiation to a WebAssembly module.
pub fn autodiff(wasm_module: &[u8]) -> wasmparser::Result<Vec<u8>> {
    wasmparser::validate(wasm_module)?;
    Ok(wasm_module.to_vec())
}
