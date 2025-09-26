#!/usr/bin/env -S uv run

import argparse
import gzip
import os
import shutil
import subprocess
import sys
from pathlib import Path


def run(cmd: list[str], **kwargs) -> None:
    subprocess.run(cmd, check=True, **kwargs)


def compile() -> str:
    run(
        [
            "cargo",
            "build",
            "--package=floretta-wasm",
            "--target=wasm32-unknown-unknown",
            "--profile=tiny",
            "-Zbuild-std=std,panic_abort",
            "-Zbuild-std-features=optimize_for_size",
        ],
        env=os.environ | {"RUSTFLAGS": "-Zunstable-options -Cpanic=immediate-abort"},
    )
    return "target/wasm32-unknown-unknown/tiny/floretta_wasm.wasm"


def print_sizes(files: dict[str, int], *, out) -> None:
    m = max(len(k) for k in files.keys())
    n = max(len(str(v)) for v in files.values())
    print("```", file=out)
    for k, v in files.items():
        print(f"{k:<{m}} is {v:>{n}} bytes", file=out)
    print("```", file=out)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "-o", "--output", type=argparse.FileType("a"), default=sys.stdout
    )
    args = parser.parse_args()
    name = "floretta.wasm"
    shutil.copy(compile(), name)
    wasm = Path(name).read_bytes()
    gz = gzip.compress(wasm)
    print_sizes({name: len(wasm), f"{name}.gz": len(gz)}, out=args.output)


if __name__ == "__main__":
    main()
