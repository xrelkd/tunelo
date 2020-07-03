with import <nixpkgs> { };
stdenv.mkDerivation {
  name = "tunelo-dev";
  buildInputs = [ rustup ];
}
