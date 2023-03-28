{ cargoArgs
, unitTestArgs
, pkgs
, writeShellScriptBin
, ...
}:

let
  cargo-ext = pkgs.callPackage ./cargo-ext.nix { inherit cargoArgs unitTestArgs; };
in
pkgs.mkShell rec {
  name = "dev-shell";

  nativeBuildInputs = with pkgs; [
    cargo-ext.cargo-build-all
    cargo-ext.cargo-clippy-all
    cargo-ext.cargo-doc-all
    cargo-ext.cargo-nextest-all
    cargo-ext.cargo-test-all
    cargo-ext.cargo-udeps-all
    cargo-ext.cargo-watch-all

    (fenix.default.withComponents [
      "cargo"
      "rustc"
      "clippy"
      "rustfmt"
    ])
    cargo-nextest
    cargo-udeps
    cargo-watch

    tokei

    treefmt

    jq
    nixpkgs-fmt
    shfmt
    nodePackages.prettier
    shellcheck
  ];

  shellHook = ''
    export NIX_PATH="nixpkgs=${pkgs.path}"
    export PATH=$PWD/dev-support/bin:$PATH
  '';
}
