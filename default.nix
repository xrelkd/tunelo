{ lib
, rustPlatform
, installShellFiles
}:

rustPlatform.buildRustPackage rec {
  pname = "tunelo";
  version = "0.1.8";

  src = lib.cleanSource ./.;

  cargoLock.lockFile = ./Cargo.lock;

  nativeBuildInputs = [ installShellFiles ];

  postInstall = ''
    installShellCompletion --cmd tunelo \
      --bash <($out/bin/tunelo completions bash) \
      --fish <($out/bin/tunelo completions fish) \
      --zsh  <($out/bin/tunelo completions zsh)
  '';

  meta = with lib; {
    homepage = "https://github.com/xrelkd/tunelo";
    license = with licenses; [ gpl3Only ];
    maintainers = with maintainers; [ xrelkd ];
  };
}
