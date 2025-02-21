#[unsafe(no_mangle)]
fn forward(wasm: &[u8]) -> Result<Vec<u8>, floretta::Error> {
    floretta::Forward::no_validate().transform(wasm)
}

#[unsafe(no_mangle)]
fn reverse(wasm: &[u8]) -> Result<Vec<u8>, floretta::Error> {
    floretta::Reverse::no_validate().transform(wasm)
}
