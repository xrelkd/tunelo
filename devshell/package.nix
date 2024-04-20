{ name
, version
, lib
, stdenv
, rustPlatform
, installShellFiles
, darwin
}:

rustPlatform.buildRustPackage {
  pname = name;
  inherit version;

  src = lib.cleanSource ./..;

  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [ installShellFiles ];

  buildInputs = lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.Security
    darwin.apple_sdk.frameworks.SystemConfiguration
  ];

  postInstall = ''
    installShellCompletion --cmd tunelo \
      --bash <($out/bin/tunelo completions bash) \
      --fish <($out/bin/tunelo completions fish) \
      --zsh  <($out/bin/tunelo completions zsh)
  '';

  doCheck = false;

  meta = with lib; {
    homepage = "https://github.com/xrelkd/tunelo";
    license = with licenses; [ gpl3Only ];
    maintainers = with maintainers; [ xrelkd ];
  };
}
