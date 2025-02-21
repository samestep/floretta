#!/usr/bin/env -S uv run
# /// script
# dependencies = ["tomlkit"]
# ///

import argparse
import subprocess
from pathlib import Path

import tomlkit


def update_version(version: str) -> None:
    p = Path("Cargo.toml")
    cargo_toml = tomlkit.parse(p.read_text())
    workspace = cargo_toml["workspace"]
    workspace["package"]["version"] = version
    workspace["dependencies"]["floretta"]["version"] = f"={version}"
    p.write_text(tomlkit.dumps(cargo_toml))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("version")
    args = parser.parse_args()
    version = args.version.removeprefix("v")
    update_version(version)
    if subprocess.run(["git", "status", "--porcelain"], capture_output=True).stdout:
        raise Exception("won't create a release with uncommitted changes")
    subprocess.run(["git", "commit", "-m", f"Release v{version}"])
    subprocess.run(["git", "push"])
    subprocess.run(["gh", "release", "create", f"v{version}"])


if __name__ == "__main__":
    main()
