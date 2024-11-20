#[no_mangle]
fn autodiff(wasm_module: &[u8]) -> Result<Vec<u8>, floretta::Error> {
    floretta::Autodiff::no_validate().transform(wasm_module)
}
