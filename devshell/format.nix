{ pkgs, }:

pkgs.runCommand "check-format"
{
  buildInputs = with pkgs; [
    fd
    nixpkgs-fmt
    nodePackages.prettier
    shellcheck
    shfmt
  ];
} ''
  echo "Checking Nix format with \`nixpkgs-fmt\`"
  nixpkgs-fmt --check ${./..}
  echo

  echo "Checking shell script format with \`shfmt\`"
  shfmt -d ${./..}
  echo

  echo "Checking shell script with \`shellcheck\`"
  shfmt -f ${./..} | xargs shellcheck -s bash
  echo

  echo "Checking JavaScript, TypeScript, Markdown, JSON, YAML format with \`prettier\`"
  fd --glob '**/*.{css,html,js,json,jsx,md,mdx,scss,ts,yaml}' ${./..} | xargs prettier --check
  echo

  # it worked!
  touch $out
''
