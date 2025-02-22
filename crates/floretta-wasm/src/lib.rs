use floretta::Autodiff;

#[unsafe(no_mangle)]
fn forward(wasm: &[u8]) -> Result<Vec<u8>, floretta::Error> {
    Autodiff::no_validate().forward(wasm)
}

#[unsafe(no_mangle)]
fn reverse(wasm: &[u8]) -> Result<Vec<u8>, floretta::Error> {
    Autodiff::no_validate().reverse(wasm)
}
