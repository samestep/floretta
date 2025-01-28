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

Use this command to run all the tests:

```sh
cargo test
```

## Release

To make a new release, first push a commit updating the version number:

```sh
uv run .github/version.py $FLORETTA_VERSION
git commit -m "Release v$FLORETTA_VERSION"
git push
```

Then create a [new GitHub release][], choosing the option to **Create new tag: `v$FLORETTA_VERSION` on publish**, and using the same `v$FLORETTA_VERSION` as the release title.

[github cli]: https://cli.github.com/
[new github release]: https://github.com/samestep/floretta/releases/new
[rust]: https://www.rust-lang.org/tools/install
[uv]: https://docs.astral.sh/uv
