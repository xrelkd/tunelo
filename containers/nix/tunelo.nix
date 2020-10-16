{ lib, rustPlatform }:

rustPlatform.buildRustPackage rec {
  name = "tunelo";

  src = builtins.fetchGit { url = ../..; };

  cargoSha256 = "09sfgk7mhf9wh6q49r1bq5l7v5n248x9qhiqa7v82g58z6bh6pqs";

  meta = with lib; {
    description = "Proxy server that supports SOCKS4a, SOCKS5 and HTTP tunnel";
    license = licenses.gpl3;
    platforms = platforms.all;
    maintainers = with maintainers; [ user ];
  };
}
