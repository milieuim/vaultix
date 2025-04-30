{
  nodes,
  lib,
  pkgs,
  package,
  identity,
  cache,
  extraPackages,
  ...
}:
let
  inherit (pkgs) writeShellScriptBin;
  inherit (lib) concatStringsSep attrValues makeBinPath;
  bin = pkgs.lib.getExe package;

  profilesArgs = concatStringsSep " " (
    map (
      v:
      "--profile"
      + " "
      + (pkgs.writeTextFile {
        name = "vaultix-material";
        text = builtins.toJSON {
          inherit (v.config.vaultix)
            beforeUserborn
            placeholder
            secrets
            settings
            templates
            ;
        };
      })
    ) (attrValues nodes)
  );

  rencCmds = "${bin} ${profilesArgs} renc --identity ${identity} --cache ${cache}";

  pathPrefix = makeBinPath extraPackages;

in
writeShellScriptBin "renc" ''
  export PATH=${pathPrefix}:$PATH
  ${rencCmds}
''
