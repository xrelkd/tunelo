with import <nixpkgs> { };
stdenv.mkDerivation {
  name = "tunelo-dev";

  RUST_BACKTRACE = 1;

  nativeBuildInputs = [ rustup ];
}
