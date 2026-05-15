{
  description = "cf-nix-cache - Cloudflare-native Nix binary cache using Workers, R2, and Rust.";

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
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rust-toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        worker-build = pkgs.rustPlatform.buildRustPackage rec {
          pname = "worker-build";
          version = "0.8.3";
          src = pkgs.fetchFromGitHub {
            owner = "cloudflare";
            repo = "workers-rs";
            rev = "v${version}";
            fetchSubmodules = true;
            hash = "sha256-sRKQALNYUmzxaqYJCWR8b3yvqg8e4EHe1Cm7vqRx8hU=";
          };
          cargoHash = "sha256-enePrsTLpiTDxqnFFD38N4amOKY5oHHctPl9RFj2eRo=";
          buildAndTestSubdir = "worker-build";
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.openssl ];
          doCheck = false;
        };
      in
      {
        devShells.default = pkgs.mkShell {
          name = "cf-nix-cache";
          packages = [
            pkgs.pkg-config
            pkgs.wrangler
            rust-toolchain
            worker-build
          ];
        };
      }
    );
}
