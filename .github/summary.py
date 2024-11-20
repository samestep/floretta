#!/usr/bin/env python3

from pathlib import Path


def main():
    size = Path("floretta.wasm").stat().st_size
    print(f"`floretta.wasm` is {size} bytes.")


if __name__ == "__main__":
    main()
