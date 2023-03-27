{ name
, version
, dockerTools
, tunelo
, bashInteractive
, buildEnv
, ...
}:

dockerTools.buildImage {
  inherit name;
  tag = "v${version}";

  copyToRoot = buildEnv {
    name = "image-root";
    paths = [ tunelo bashInteractive ];
    pathsToLink = [ "/bin" ];
  };

  config = {
    Entrypoint = [ "${tunelo}/bin/tunelo" "socks-server" ];
  };
}
