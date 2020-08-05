let
  sources = import ./nix/sources.nix;
  pkgs = import sources.nixpkgs { };
  inherit (pkgs) stdenv;
in pkgs.mkShell {
  nativeBuildInputs = with pkgs; [

    git
    rustup
    cargo-make
  ];

  RUST_BACKTRACE = 1;
}
