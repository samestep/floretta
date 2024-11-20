#!/usr/bin/env python3

import os


def main():
    size = os.path.getsize("floretta.wasm")
    print(f"`floretta.wasm` is {size} bytes.")


if __name__ == "__main__":
    main()
