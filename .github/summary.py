#!/usr/bin/env python3

import gzip
from pathlib import Path


def print_sizes(files: dict[str, int]) -> None:
    m = max(len(k) for k in files.keys())
    n = max(len(str(v)) for v in files.values())
    print("```")
    for k, v in files.items():
        print(f"{k:<{m}} is {v:>{n}} bytes")
    print("```")


def main() -> None:
    name = "floretta.wasm"
    wasm = Path(name).read_bytes()
    gz = gzip.compress(wasm)
    print_sizes({name: len(wasm), f"{name}.gz": len(gz)})


if __name__ == "__main__":
    main()
