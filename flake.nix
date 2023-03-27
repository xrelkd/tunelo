{
  description = "Tunelo -  Proxy server that supports SOCKS4a, SOCKS5 and HTTP tunnel";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix?ref=d143afc6110296af610d7f77f54808e946d2e62d";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, fenix, crane }:
    (flake-utils.lib.eachDefaultSystem

      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [
              self.overlays.default
              fenix.overlays.default
            ];
          };

          craneLib = (crane.mkLib pkgs).overrideToolchain
            (pkgs.fenix.default.withComponents [
              "cargo"
              "rustc"
              "clippy"
              "rustfmt"
            ]);

          src = craneLib.cleanCargoSource (craneLib.path ./.);
          commonArgs = {
            inherit src;
          };
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        in
        rec {
          formatter = pkgs.treefmt;
          devShells.default = pkgs.callPackage ./devshell { };

          packages = rec {
            default = tunelo;
            tunelo = pkgs.callPackage ./default.nix { };
          };

          apps.default = flake-utils.lib.mkApp {
            drv = packages.tunelo;
            exePath = "/bin/tunelo";
          };

          checks = {
            format = pkgs.callPackage ./devshell/format.nix { };

            rust-format = craneLib.cargoFmt { inherit src; };
            rust-clippy = craneLib.cargoClippy (commonArgs // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets --workspace --bins --examples --tests --benches";
            });
            rust-nextest = craneLib.cargoNextest (commonArgs // {
              inherit cargoArtifacts;
              partitions = 1;
              partitionType = "count";
            });
          };
        })) // {
      overlays.default = final: prev: {
        tunelo = final.callPackage ./default.nix { };
      };
    };
}
