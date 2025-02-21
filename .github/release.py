#!/usr/bin/env -S uv run
# /// script
# dependencies = ["tomlkit"]
# ///

import argparse
import subprocess
from pathlib import Path

import tomlkit


def run(cmd: list[str]) -> None:
    subprocess.run(cmd, check=True)


def update_version(path: Path, version: str) -> None:
    cargo_toml = tomlkit.parse(path.read_text())
    workspace = cargo_toml["workspace"]
    workspace["package"]["version"] = version
    workspace["dependencies"]["floretta"]["version"] = f"={version}"
    path.write_text(tomlkit.dumps(cargo_toml))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("version")
    args = parser.parse_args()
    version = args.version.removeprefix("v")
    path = Path("Cargo.toml")
    update_version(path, version)
    run(["git", "add", path])
    run(["git", "commit", "-m", f"Release v{version}"])
    run(["git", "push"])
    run(["gh", "release", "create", f"v{version}", "--title", f"v{version}"])


if __name__ == "__main__":
    main()
