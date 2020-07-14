{ lib, rustPlatform }:

rustPlatform.buildRustPackage rec {
  name = "tunelo";

  src = builtins.fetchGit { url = ../..; };

  cargoSha256 = "0yvf6xc1ds96ry5vpykp8izv4z3jszw02mb5r9ldwxb5v2iba2wj";

  meta = with lib; {
    description = "Proxy server that supports SOCKS4a, SOCKS5 and HTTP tunnel";
    license = licenses.gpl3;
    platforms = platforms.all;
    maintainers = with maintainers; [ user ];
  };
}
