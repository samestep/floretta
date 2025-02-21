# Floretta

[Floretta][] is an [automatic differentiation][] tool for [WebAssembly][]. This crate is the Rust library; for the command line interface, see the [`floretta-cli`][] crate.

To install:

```sh
cargo add floretta
```

Here are some usage examples, assuming you have [`wat`][] and [Wasmtime][] installed.

## Forward mode

Use `Forward::new` to create an empty config, then use `Forward::transform` to process a Wasm module.

```rust
use wasmtime::{Engine, Instance, Module, Store};

let input = wat::parse_str(r#"
(module
  (func (export "square") (param f64) (result f64)
    (f64.mul (local.get 0) (local.get 0))))
"#).unwrap();

let ad = floretta::Forward::new();
let output = ad.transform(&input).unwrap();

let engine = Engine::default();
let mut store = Store::new(&engine, ());
let module = Module::new(&engine, &output).unwrap();
let instance = Instance::new(&mut store, &module, &[]).unwrap();
let square = instance.get_typed_func::<(f64, f64), (f64, f64)>(&mut store, "square").unwrap();

assert_eq!(square.call(&mut store, (3., 1.)).unwrap(), (9., 6.));
```

## Reverse mode

Create an empty config via `Reverse::new`, use `Reverse::export` to specify one or more functions to export the backward pass, and then use `Reverse::transform` to process a Wasm module.

```rust
use wasmtime::{Engine, Instance, Module, Store};

let input = wat::parse_str(r#"
(module
  (func (export "square") (param f64) (result f64)
    (f64.mul (local.get 0) (local.get 0))))
"#).unwrap();

let mut ad = floretta::Reverse::new();
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
