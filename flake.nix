{
  description = "Tunelo -  Proxy server that supports SOCKS4a, SOCKS5 and HTTP tunnel";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    (flake-utils.lib.eachDefaultSystem

      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ self.overlay ];
          };
        in
        rec {
          packages.tunelo = pkgs.callPackage ./default.nix { };
          defaultPackage = packages.tunelo;
          apps.default = flake-utils.lib.mkApp {
            drv = packages.tunelo;
            exePath = "/bin/tunelo";
          };
          devShell = pkgs.callPackage ./shell.nix { };

        })) // {
      overlay = final: prev: {
        tunelo = final.callPackage ./default.nix { };
      };
    };
}
