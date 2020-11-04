{ lib, rustPlatform }:

rustPlatform.buildRustPackage rec {
  name = "tunelo";

  src = builtins.fetchGit { url = ../..; };

  cargoSha256 = "1s379cnljhrarj6r2xif2vrqqb3nfk16qz951jyfb8a3i6crq6c3";

  meta = with lib; {
    description = "Proxy server that supports SOCKS4a, SOCKS5 and HTTP tunnel";
    license = licenses.gpl3;
    platforms = platforms.all;
    maintainers = with maintainers; [ user ];
  };
}
