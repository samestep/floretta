# Contributing to Floretta

## Prerequisites

Be sure to have these tools installed:

- [GitHub CLI][]
- [Rust][] nightly with the [`rust-src` component][rust-src]
- [uv][]

## Setup

Clone this repository, and open a terminal in that clone:

```sh
gh repo clone samestep/floretta
cd floretta
```

## Testing

To run all the tests:

```sh
cargo test
```

## Wasm

To compile Floretta itself into a Wasm binary:

```sh
.github/wasm.py
```

## Release

To make a new release:

```sh
.github/release.py $FLORETTA_VERSION
```

[github cli]: https://cli.github.com/
[rust]: https://www.rust-lang.org/tools/install
[rust-src]: https://rust-lang.github.io/rustup/concepts/components.html
[uv]: https://docs.astral.sh/uv
