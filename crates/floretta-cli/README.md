# Floretta CLI

[Floretta][] is an [automatic differentiation][] tool for [WebAssembly][]. This crate is the command line interface; for the Rust library, see the [`floretta`][] crate.

To install:

```sh
cargo install --locked floretta-cli
```

Use the `--help` flag to see all available CLI arguments:

```sh
floretta --help
```

Here are some usage examples, assuming you have [Node.js][] installed. First, create a file called `square.wat` with these contents:

```wat
(module
  (func (export "square") (param f64) (result f64)
    (f64.mul (local.get 0) (local.get 0))))
```

## Forward mode

You can use Floretta to replace the `"square"` function approximating real numbers with one approximating the [dual numbers][]:

```sh
floretta --forward square.wat --output dual.wasm
```

Then if you have a Wasm engine, you can use it to compute the der with the emitted Wasm binary by running the augmented function with a value of 1 for the dual part. For instance, you can create a file called `dual.mjs` with these contents:

```js
import fs from "node:fs/promises";
const wasm = await fs.readFile("dual.wasm");
const module = await WebAssembly.instantiate(wasm);
const { square } = module.instance.exports;
console.log(square(3, 1));
```

And run it like this:

```sh
node dual.mjs
```

Expected output:

```
[ 9, 6 ]
```

## Reverse mode

You can use Floretta to take the backward pass of the `"square"` function and name it `"backprop"`:

```sh
floretta --reverse square.wat --export square backprop --output gradient.wasm
```

Then if you have a Wasm engine, you can use it on the emitted Wasm binary to compute a gradient by running the forward pass followed by the backward pass. For instance, you can create a file called `gradient.mjs` with these contents:

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

[`floretta`]: https://crates.io/crates/floretta
[automatic differentiation]: https://en.wikipedia.org/wiki/Automatic_differentiation
[dual numbers]: https://en.wikipedia.org/wiki/Dual_number
[floretta]: https://github.com/samestep/floretta
[node.js]: https://nodejs.org
[webassembly]: https://webassembly.org/
