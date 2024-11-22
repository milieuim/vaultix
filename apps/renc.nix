{
  nodes,
  lib,
  pkgs,
  package,
  identity,
  cache,
  ...
}:
let
  inherit (pkgs) writeShellScriptBin;
  inherit (lib) concatStringsSep attrValues;
  bin = pkgs.lib.getExe package;

  profilesArgs = concatStringsSep " " (
    map (
      v:
      "--profile"
      + " "
      + (pkgs.writeTextFile {
        name = "vaultix-material";
        text = builtins.toJSON v.config.vaultix;
      })
    ) (attrValues nodes)
  );

  rencCmds = "${bin} ${profilesArgs} renc --identity ${identity} --cache ${cache}";

in
writeShellScriptBin "renc" rencCmds
