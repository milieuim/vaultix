{
  pkgs,
  package,
  identity,
  extraRecipients,
  extraPackages,
  pinentryPackage,
  ...
}:
let
  inherit (pkgs) writeShellScriptBin;
  inherit (pkgs.lib)
    concatStringsSep
    makeBinPath
    optionalString
    getExe
    ;

  bin = getExe package;
  recipientsArg = concatStringsSep " " (map (n: "--recipient ${n}") extraRecipients);
  pathPrefix = makeBinPath extraPackages;

in
writeShellScriptBin "edit-secret" ''
  export PATH=${pathPrefix}:$PATH
  ${optionalString (pinentryPackage != null) "export PINENTRY_PROGRAM=${getExe pinentryPackage}"}
  ${bin} edit --identity ${identity} ${recipientsArg} $1
''
