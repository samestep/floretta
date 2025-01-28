# Floretta [![crates.io](https://img.shields.io/crates/v/floretta)][crate] [![docs.rs](https://img.shields.io/docsrs/floretta)][docs] [![Build](https://github.com/samestep/floretta/actions/workflows/build.yml/badge.svg)](https://github.com/samestep/floretta/actions/workflows/build.yml)

Take the [gradient][] of any [Wasm][] function. _**(Warning: work in progress.)**_

Floretta uses reverse-mode [automatic differentiation][] to transform a Wasm module, converting every function into two:

1. The _forward pass_, which performs the same computation as the original function, but stores some extra data along the way, referred to as the _tape_.
2. The _backward pass_, which uses the tape stored by the forward pass to retrace its steps in reverse.

Together, these comprise the _vector-Jacobian product (VJP)_, which can then be used to compute the gradient of any function that returns a scalar.

For every memory in the original Wasm module, Floretta adds an additional memory in the transformed module, to store the derivative of each scalar in the original memory. Also, Floretta adds two more memories to store the tape: one for `f32` values, and one for `f64` values.

## Usage

The easiest way to use Floretta is via the command line. If you have [Rust][] installed, you can build the latest version of Floretta from source:

```sh
cargo install --locked floretta-cli
```

Use the `--help` flag to see all available CLI arguments:

```sh
floretta --help
```

For example, if you create a file called `square.wat` with these contents:

```wat
(module
  (func (export "square") (param f64) (result f64)
    (f64.mul (local.get 0) (local.get 0))))
```

Then you can use Floretta to take the backward pass of the `"square"` function and name it `"backprop"`:

```sh
floretta square.wat --export square backprop --output gradient.wasm
```

Finally, if you have a Wasm engine, you can use it to compute a gradient with the emitted Wasm binary by running the forward pass followed by the backward pass. For instance, if you have [Node.js][] installed, you can create a file called `gradient.mjs` with these contents:

```js
import fs from "node:fs/promises";
const wasm = await fs.readFile("gradient.wasm");
const module = await WebAssembly.instantiate(wasm);
const { square, backprop } = module.instance.exports;
console.log(square(3));
console.log(backprop(1));
```

And run it like this:

```sh
node gradient.mjs
```

Expected output:

```
9
6
```

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md).

## License

Floretta is licensed under the [MIT License](LICENSE).

[automatic differentiation]: https://en.wikipedia.org/wiki/Automatic_differentiation
[crate]: https://crates.io/crates/floretta
[docs]: https://docs.rs/floretta
[gradient]: https://en.wikipedia.org/wiki/Gradient
[node.js]: https://nodejs.org
[rust]: https://www.rust-lang.org/tools/install
[wasm]: https://webassembly.org/
