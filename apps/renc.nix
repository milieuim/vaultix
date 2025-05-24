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
  inherit (lib)
    concatStringsSep
    attrValues
    makeBinPath
    throwIfNot
    ;
  bin = pkgs.lib.getExe package;

  profilesArgs = concatStringsSep " " (
    map (
      v:
      "--profile"
      + " "
      + (pkgs.writeTextFile {
        name = "vaultix-material";
        text =
          throwIfNot (v.config ? vaultix)
            ''
              Host ${v.config.networking.hostName} doesn't had vaultix nixosModule imported.
              Check your configuration, or remove it from `vaultix.nodes`.
              If it's fresh setup please follow <https://milieuim.github.io/vaultix/nixos-option.html>
            ''
            builtins.toJSON
            {
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
