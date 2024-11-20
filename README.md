# Floretta [![crates.io](https://img.shields.io/crates/v/floretta)][crate] [![docs.rs](https://img.shields.io/docsrs/floretta)][docs] [![Build](https://github.com/samestep/floretta/actions/workflows/build.yml/badge.svg)](https://github.com/samestep/floretta/actions/workflows/build.yml)

Take the [gradient][] of any [Wasm][] function. _**(Warning: work in progress.)**_

Floretta uses reverse-mode [automatic differentiation][] to transform a Wasm module, converting every function into two:

1. The _forward pass_, which performs the same computation as the original function, but stores some extra data along the way, referred to as the _tape_.
2. The _backward pass_, which uses the tape stored by the forward pass to retrace its steps in reverse.

Together, these comprise the _vector-Jacobian product (VJP)_, which can then be used to compute the gradient of any function that returns a scalar.

For every memory in the original Wasm module, Floretta adds an additional memory in the transformed module, to store the derivative of each scalar in the original memory. Also, Floretta adds one last memory to store the tape.

## Usage

The easiest way to use Floretta is via the command line. If you have [Rust][] installed, you can build the latest version of Floretta from source:

```
$ cargo install --locked floretta-cli
```

Use the `--help` flag to see all available CLI arguments:

```
$ floretta --help
```

For example, if you create a file called `square.wat` with these contents:

```wat
(module
  (func (export "square") (param f64) (result f64)
    (f64.mul (local.get 0) (local.get 0))))
```

Then you can use Floretta to take the backward pass of the `"square"` function and name it `"double"`, since the gradient of $f(x) = x^2$ is $\nabla f(x) = 2x$ which doubles its argument:

```
$ floretta square.wat --export square double --output double.wasm
```

Finally, if you have a Wasm engine like [Wasmtime][] installed, you can use it to run that gradient function in the emitted Wasm binary:

```
$ wasmtime --invoke double double.wasm 3
warning: using `--invoke` with a function that takes arguments is experimental and may break in the future
warning: using `--invoke` with a function that returns values is experimental and may break in the future
6
```

## License

Floretta is licensed under the [MIT License](LICENSE).

[automatic differentiation]: https://en.wikipedia.org/wiki/Automatic_differentiation
[crate]: https://crates.io/crates/floretta
[docs]: https://docs.rs/floretta
[gradient]: https://en.wikipedia.org/wiki/Gradient
[rust]: https://www.rust-lang.org/tools/install
[wasm]: https://webassembly.org/
[wasmtime]: https://wasmtime.dev/
