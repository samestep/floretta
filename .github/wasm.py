#!/usr/bin/env -S uv run

import gzip
import shutil
import subprocess
from pathlib import Path


def run(cmd: list[str]) -> None:
    subprocess.run(cmd, check=True)


def compile() -> str:
    run(
        [
            "cargo",
            "+nightly",
            "build",
            "--package=floretta-wasm",
            "--target=wasm32-unknown-unknown",
            "--profile=tiny",
            "-Zbuild-std=std,panic_abort",
            "-Zbuild-std-features=optimize_for_size,panic_immediate_abort",
        ]
    )
    return "target/wasm32-unknown-unknown/tiny/floretta_wasm.wasm"


def print_sizes(files: dict[str, int]) -> None:
    m = max(len(k) for k in files.keys())
    n = max(len(str(v)) for v in files.values())
    print("```")
    for k, v in files.items():
        print(f"{k:<{m}} is {v:>{n}} bytes")
    print("```")


def main() -> None:
    name = "floretta.wasm"
    shutil.copy(compile(), name)
    wasm = Path(name).read_bytes()
    gz = gzip.compress(wasm)
    print_sizes({name: len(wasm), f"{name}.gz": len(gz)})


if __name__ == "__main__":
    main()
