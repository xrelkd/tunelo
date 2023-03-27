{ lib
, writeShellScriptBin
,
}:

let
  CARGO_ARGS = [
    "--workspace"
    "--bins"
    "--examples"
    "--tests"
    "--benches"
    "--all-targets"
  ];

  UNIT_TEST_ARGS = [
    "--workspace"
  ];

  CARGO_ARGUMENTS = lib.strings.concatMapStrings (x: x + " ") CARGO_ARGS;
  UNIT_TEST_ARGUMENTS = lib.strings.concatMapStrings (x: x + " ") UNIT_TEST_ARGS;
in
{
  cargo-build-all = writeShellScriptBin "cargo-build-all" ''
    if [ $# -gt 0 ] && [ "$1" = "build-all" ]; then
      shift
    fi

    cargo version
    rustc --version
    cargo build ${CARGO_ARGUMENTS} "$@"
  '';

  cargo-clippy-all = writeShellScriptBin "cargo-clippy-all" ''
    if [ $# -gt 0 ] && [ "$1" = "clippy-all" ]; then
      shift
    fi

    cargo clippy --version
    rustc --version
    cargo clippy ${CARGO_ARGUMENTS} "$@"
  '';

  cargo-doc-all = writeShellScriptBin "cargo-doc-all" ''
    if [ $# -gt 0 ] && [ "$1" = "doc-all" ]; then
      shift
    fi

    cargo --version
    rustc --version
    cargo doc --workspace --no-deps --bins --all-features "$@"
  '';

  cargo-test-all = writeShellScriptBin "cargo-test-all" ''
    if [ $# -gt 0 ] && [ "$1" = "test-all" ]; then
      shift
    fi

    cargo --version
    rustc --version
    cargo test ${UNIT_TEST_ARGUMENTS} --no-fail-fast "$@" -- \
      --nocapture \
      --test \
      -Z unstable-options \
      --report-time
  '';

  cargo-nextest-all = writeShellScriptBin "cargo-nextest-all" ''
    if [ $# -gt 0 ] && [ "$1" = "nextest-all" ]; then
      shift
    fi

    cargo --version
    rustc --version
    cargo nextest --version
    cargo nextest run --no-capture
  '';

  cargo-watch-all = writeShellScriptBin "cargo-watch-all" ''
    if [ $# -gt 0 ] && [ "$1" = "watch-all" ]; then
      shift
    fi

    cargo --version
    rustc --version
    cargo clippy --version
    cargo watch -c -- cargo "$@" ${CARGO_ARGUMENTS}
  '';
}
