{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
      in
      {
        devShell = pkgs.mkShell {
          buildInputs = [
            pkgs.gh
            pkgs.nixfmt-rfc-style
            pkgs.nodejs
            pkgs.uv
            pkgs.wasm-tools
            (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
          ];
        };
      }
    );
}
