# /// script
# requires-python = ">=3.11"
# dependencies = [
#     "tomlkit",
# ]
# ///

import argparse
from pathlib import Path

import tomlkit


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("version")
    args = parser.parse_args()
    p = Path("Cargo.toml")
    cargo_toml = tomlkit.parse(p.read_text())
    workspace = cargo_toml["workspace"]
    workspace["package"]["version"] = args.version
    workspace["dependencies"]["floretta"]["version"] = f"={args.version}"
    p.write_text(tomlkit.dumps(cargo_toml))


if __name__ == "__main__":
    main()
