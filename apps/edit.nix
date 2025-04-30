{
  pkgs,
  package,
  identity,
  extraRecipients,
  extraPackages,
  ...
}:
let
  inherit (pkgs) writeShellScriptBin;
  inherit (pkgs.lib) concatStringsSep makeBinPath;

  bin = pkgs.lib.getExe package;
  recipientsArg = concatStringsSep " " (map (n: "--recipient ${n}") extraRecipients);
  pathPrefix = makeBinPath extraPackages;

in
writeShellScriptBin "edit-secret" ''
  export PATH=${pathPrefix}:$PATH
  ${bin} edit --identity ${identity} ${recipientsArg} $1
''
