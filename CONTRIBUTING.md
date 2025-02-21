# Contributing to Floretta

## Prerequisites

Be sure to have these tools installed:

- [GitHub CLI][]
- [Rust][]
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
[uv]: https://docs.astral.sh/uv
