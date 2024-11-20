#[no_mangle]
fn autodiff(wasm_module: &[u8]) -> Result<Vec<u8>, floretta::Error> {
    floretta::Autodiff::new().transform(wasm_module)
}
