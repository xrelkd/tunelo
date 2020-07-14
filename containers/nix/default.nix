{ pkgs ? import <nixpkgs> { }, version }:

with pkgs;
let
  tag = version;
  tunelo = callPackage ./tunelo.nix { };
in {
  tunelo = dockerTools.buildImage {
    name = "tunelo";
    inherit tag;
    created = "now";

    fromImage = null;

    contents = [ tunelo ];

    config = { Entrypoint = [ "${tunelo}/bin/tunelo" "socks-server" ]; };
  };
}
