use floretta::Autodiff;

#[no_mangle]
fn forward(wasm: &[u8]) -> Result<Vec<u8>, floretta::Error> {
    Autodiff::no_validate().forward(wasm)
}

#[no_mangle]
fn reverse(wasm: &[u8]) -> Result<Vec<u8>, floretta::Error> {
    Autodiff::no_validate().reverse(wasm)
}
