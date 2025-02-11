# Floretta

[Floretta][] is an [automatic differentiation][] tool for [WebAssembly][]. This crate is the Rust library; for the command line interface, see the [`floretta-cli`][] crate.

To install:

```sh
cargo add floretta
```

The typical workflow is to create an empty config via `Autodiff::new`, use `Autodiff::export` to specify one or more functions to export the backward pass, and then use `Autodiff::transform` to process a Wasm module.

For example, if you have [`wat`][] and [Wasmtime][] installed:

```rust
use wasmtime::{Engine, Instance, Module, Store};

let input = wat::parse_str(r#"
(module
  (func (export "square") (param f64) (result f64)
    (f64.mul (local.get 0) (local.get 0))))
"#).unwrap();

let mut ad = floretta::Autodiff::new();
ad.export("square", "backprop");
let output = ad.transform(&input).unwrap();

let engine = Engine::default();
let mut store = Store::new(&engine, ());
let module = Module::new(&engine, &output).unwrap();
let instance = Instance::new(&mut store, &module, &[]).unwrap();
let square = instance.get_typed_func::<f64, f64>(&mut store, "square").unwrap();
let backprop = instance.get_typed_func::<f64, f64>(&mut store, "backprop").unwrap();

assert_eq!(square.call(&mut store, 3.).unwrap(), 9.);
assert_eq!(backprop.call(&mut store, 1.).unwrap(), 6.);
```

[`floretta-cli`]: https://crates.io/crates/floretta-cli
[`wat`]: https://crates.io/crates/wat
[automatic differentiation]: https://en.wikipedia.org/wiki/Automatic_differentiation
[floretta]: https://github.com/samestep/floretta
[wasmtime]: https://crates.io/crates/wasmtime
[webassembly]: https://webassembly.org/
