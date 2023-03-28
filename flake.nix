{
  description = "Tunelo -  Proxy server that supports SOCKS4a, SOCKS5 and HTTP tunnel";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      # nightly-2023-03-24
      url = "github:nix-community/fenix?ref=d143afc6110296af610d7f77f54808e946d2e62d";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, fenix, crane }:
    let
      name = "tunelo";
      version = "0.1.8";
    in
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

          rustComponents = [
            "cargo"
            "clippy"
            "rust-src"
            "rustc"
            "rustfmt"
          ];

          rustToolchainStable = (pkgs.fenix.stable.withComponents rustComponents);
          rustToolchainNightly = (pkgs.fenix.complete.withComponents rustComponents);
          rustToolchain = rustToolchainNightly;

          craneLibStable = (crane.mkLib pkgs).overrideToolchain rustToolchainStable;
          craneLibNightly = (crane.mkLib pkgs).overrideToolchain rustToolchainNightly;
          craneLib = craneLibNightly;

          rustPlatformStable = pkgs.makeRustPlatform {
            cargo = rustToolchainStable;
            rustc = rustToolchainStable;
          };
          rustPlatformNightly = pkgs.makeRustPlatform {
            cargo = rustToolchainNightly;
            rustc = rustToolchainNightly;
          };
          rustPlatform = rustPlatformNightly;

          cargoArgs = [
            "--workspace"
            "--bins"
            "--examples"
            "--tests"
            "--benches"
            "--all-targets"
          ];

          unitTestArgs = [
            "--workspace"
          ];

          src = craneLib.cleanCargoSource (craneLib.path ./.);
          commonArgs = {
            inherit src;
          };
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        in
        rec {
          formatter = pkgs.treefmt;

          devShells.default = pkgs.callPackage ./devshell {
            inherit rustToolchain cargoArgs unitTestArgs;
          };

          packages = rec {
            default = tunelo;
            tunelo = pkgs.callPackage ./devshell/package.nix {
              inherit name version;
              rustPlatform = rustPlatformStable;
            };
            container = pkgs.callPackage ./devshell/container.nix {
              inherit name version tunelo;
            };
          };

          apps.default = flake-utils.lib.mkApp {
            drv = packages.tunelo;
            exePath = "/bin/tunelo";
          };

          checks = {
            format = pkgs.callPackage ./devshell/format.nix { };

            rust-build = craneLibStable.cargoBuild (commonArgs // {
              inherit cargoArtifacts;
            });
            rust-format = craneLib.cargoFmt { inherit src; };
            rust-clippy = craneLib.cargoClippy (commonArgs // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = pkgs.lib.strings.concatMapStrings (x: x + " ") cargoArgs;
            });
            rust-nextest = craneLib.cargoNextest (commonArgs // {
              inherit cargoArtifacts;
              partitions = 1;
              partitionType = "count";
            });
          };
        })) // {
      overlays.default = final: prev: {
        tunelo = final.callPackage ./devshell/package.nix {
          inherit name version;
        };
      };
    };
}
