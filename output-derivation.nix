# cp the rekeyed secrets in cache path to construct a derivation.
# The implementation that i referred to: <https://github.com/oddlama/agenix-rekey/blob/main/nix/output-derivation.nix>
# TODO: https://github.com/NixOS/nix/issues/6697
{
  cache,
  pkgs, # use pkgs from machine which running the app.
}:
pkgs.runCommandNoCCLocal "vaultix-cache" { buildInputs = [ pkgs.coreutils ]; } ''
  set -euo pipefail
  mkdir $out
  echo "copying path '${cache}' to store"
  cp -rv "${cache}" $out
''
