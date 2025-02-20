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
//! ad.export("square", "backprop");
//! let output = ad.transform(&input).unwrap();
//!
//! let engine = Engine::default();
//! let mut store = Store::new(&engine, ());
//! let module = Module::new(&engine, &output).unwrap();
//! let instance = Instance::new(&mut store, &module, &[]).unwrap();
//! let square = instance.get_typed_func::<f64, f64>(&mut store, "square").unwrap();
//! let backprop = instance.get_typed_func::<f64, f64>(&mut store, "backprop").unwrap();
//!
//! assert_eq!(square.call(&mut store, 3.).unwrap(), 9.);
//! assert_eq!(backprop.call(&mut store, 1.).unwrap(), 6.);
//! ```
//!
//! [`wat`]: https://crates.io/crates/wat
//! [wasmtime]: https://crates.io/crates/wasmtime

mod api;
mod helper;
#[cfg(feature = "names")]
mod name;
mod transform;
mod util;
mod validate;

use wasm_encoder::reencode;
use wasmparser::BinaryReaderError;

pub use api::*;

#[derive(Debug, thiserror::Error)]
enum ErrorImpl {
    #[error("Wasm parsing or validation error: {0}")]
    Parse(#[from] BinaryReaderError),

    #[error("Wasm reencoding error: {0}")]
    Reencode(#[from] reencode::Error),
}

type Result<T> = std::result::Result<T, ErrorImpl>;

#[cfg(test)]
mod tests {
    use std::io::Write;

    use goldenfile::Mint;
    use wasmtime::{Engine, Instance, Module, Store};

    #[test]
    #[cfg(feature = "names")]
    fn test_names() {
        let input = wat::parse_str(include_str!("wat/names.wat")).unwrap();
        let mut ad = crate::Autodiff::new();
        ad.names();
        let output = wasmprinter::print_bytes(ad.transform(&input).unwrap()).unwrap();
        let mut mint = Mint::new("src/out");
        let mut file = mint.new_goldenfile("names.wat").unwrap();
        file.write_all(output.as_bytes()).unwrap();
    }

    #[test]
    fn test_square() {
        let input = wat::parse_str(include_str!("wat/square.wat")).unwrap();

        let mut ad = crate::Autodiff::new();
        ad.export("square", "backprop");
        let output = ad.transform(&input).unwrap();

        let engine = Engine::default();
        let mut store = Store::new(&engine, ());
        let module = Module::new(&engine, &output).unwrap();
        let instance = Instance::new(&mut store, &module, &[]).unwrap();
        let square = instance
            .get_typed_func::<f64, f64>(&mut store, "square")
            .unwrap();
        let backprop = instance
            .get_typed_func::<f64, f64>(&mut store, "backprop")
            .unwrap();

        assert_eq!(square.call(&mut store, 3.).unwrap(), 9.);
        assert_eq!(backprop.call(&mut store, 1.).unwrap(), 6.);
    }

    #[test]
    fn test_tuple() {
        let input = wat::parse_str(include_str!("wat/tuple.wat")).unwrap();

        let mut ad = crate::Autodiff::new();
        ad.export("tuple", "backprop");
        let output = ad.transform(&input).unwrap();

        let engine = Engine::default();
        let mut store = Store::new(&engine, ());
        let module = Module::new(&engine, &output).unwrap();
        let instance = Instance::new(&mut store, &module, &[]).unwrap();
        let fwd = instance
            .get_typed_func::<(i32, f64, i64, f32), (f32, i32, f64, i64)>(&mut store, "tuple")
            .unwrap();
        let bwd = instance
            .get_typed_func::<(f32, i32, f64, i64), (i32, f64, i64, f32)>(&mut store, "backprop")
            .unwrap();

        assert_eq!(
            fwd.call(&mut store, (1, 2., 3, 4.)).unwrap(),
            (4., 1, 2., 3),
        );
        assert_eq!(
            bwd.call(&mut store, (5., 6, 7., 8)).unwrap(),
            (0, 7., 0, 5.),
        );
    }

    #[test]
    fn test_loop() {
        let input = wat::parse_str(include_str!("wat/loop.wat")).unwrap();

        let mut ad = crate::Autodiff::new();
        ad.export("loop", "backprop");
        let output = ad.transform(&input).unwrap();

        let engine = Engine::default();
        let mut store = Store::new(&engine, ());
        let module = Module::new(&engine, &output).unwrap();
        let instance = Instance::new(&mut store, &module, &[]).unwrap();
        let fwd = instance
            .get_typed_func::<f64, f64>(&mut store, "loop")
            .unwrap();
        let bwd = instance
            .get_typed_func::<f64, f64>(&mut store, "backprop")
            .unwrap();

        assert_eq!(fwd.call(&mut store, 1.1).unwrap(), -0.99);
        assert_eq!(bwd.call(&mut store, 1.).unwrap(), 0.20000000000000018);
    }

    #[test]
    fn test_f32_mul() {
        let input = wat::parse_str(include_str!("wat/f32_mul.wat")).unwrap();

        let mut ad = crate::Autodiff::new();
        ad.export("mul", "backprop");
        let output = ad.transform(&input).unwrap();

        let engine = Engine::default();
        let mut store = Store::new(&engine, ());
        let module = Module::new(&engine, &output).unwrap();
        let instance = Instance::new(&mut store, &module, &[]).unwrap();
        let fwd = instance
            .get_typed_func::<(f32, f32), f32>(&mut store, "mul")
            .unwrap();
        let bwd = instance
            .get_typed_func::<f32, (f32, f32)>(&mut store, "backprop")
            .unwrap();

        assert_eq!(fwd.call(&mut store, (3., 2.)).unwrap(), 6.);
        assert_eq!(bwd.call(&mut store, 1.).unwrap(), (2., 3.));
    }

    #[test]
    fn test_f32_div() {
        let input = wat::parse_str(include_str!("wat/f32_div.wat")).unwrap();

        let mut ad = crate::Autodiff::new();
        ad.export("div", "backprop");
        let output = ad.transform(&input).unwrap();

        let engine = Engine::default();
        let mut store = Store::new(&engine, ());
        let module = Module::new(&engine, &output).unwrap();
        let instance = Instance::new(&mut store, &module, &[]).unwrap();
        let fwd = instance
            .get_typed_func::<(f32, f32), f32>(&mut store, "div")
            .unwrap();
        let bwd = instance
            .get_typed_func::<f32, (f32, f32)>(&mut store, "backprop")
            .unwrap();

        assert_eq!(fwd.call(&mut store, (3., 2.)).unwrap(), 1.5);
        assert_eq!(bwd.call(&mut store, 1.).unwrap(), (0.5, -0.75));
    }

    #[test]
    fn test_f64_mul() {
        let input = wat::parse_str(include_str!("wat/f64_mul.wat")).unwrap();

        let mut ad = crate::Autodiff::new();
        ad.export("mul", "backprop");
        let output = ad.transform(&input).unwrap();

        let engine = Engine::default();
        let mut store = Store::new(&engine, ());
        let module = Module::new(&engine, &output).unwrap();
        let instance = Instance::new(&mut store, &module, &[]).unwrap();
        let fwd = instance
            .get_typed_func::<(f64, f64), f64>(&mut store, "mul")
            .unwrap();
        let bwd = instance
            .get_typed_func::<f64, (f64, f64)>(&mut store, "backprop")
            .unwrap();

        assert_eq!(fwd.call(&mut store, (3., 2.)).unwrap(), 6.);
        assert_eq!(bwd.call(&mut store, 1.).unwrap(), (2., 3.));
    }

    #[test]
    fn test_f64_div() {
        let input = wat::parse_str(include_str!("wat/f64_div.wat")).unwrap();

        let mut ad = crate::Autodiff::new();
        ad.export("div", "backprop");
        let output = ad.transform(&input).unwrap();

        let engine = Engine::default();
        let mut store = Store::new(&engine, ());
        let module = Module::new(&engine, &output).unwrap();
        let instance = Instance::new(&mut store, &module, &[]).unwrap();
        let fwd = instance
            .get_typed_func::<(f64, f64), f64>(&mut store, "div")
            .unwrap();
        let bwd = instance
            .get_typed_func::<f64, (f64, f64)>(&mut store, "backprop")
            .unwrap();

        assert_eq!(fwd.call(&mut store, (3., 2.)).unwrap(), 1.5);
        assert_eq!(bwd.call(&mut store, 1.).unwrap(), (0.5, -0.75));
    }
}
