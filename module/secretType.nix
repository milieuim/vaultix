{
  lib,
  cfg,
  users,
  ...
}:

let
  inherit (lib)
    types
    elem
    mkOption
    literalExpression
    ;
in
{
  secretType = types.submodule (submod: {
    options = {
      id = mkOption {
        type = types.str;
        default = submod.config._module.args.name;
        readOnly = true;
        description = "The true identifier of this secret as used in `age.secrets`.";
      };
      name = mkOption {
        type = types.str;
        default = submod.config._module.args.name;
        defaultText = literalExpression "submod.config._module.args.name";
        description = ''
          Name of the file used in {option}`vaultix.settings.decryptedDir`
        '';
      };
      file = mkOption {
        type = types.path;
        description = ''
          Age file the secret is loaded from.
        '';
      };
      path = mkOption {
        type = types.str;
        default =
          if elem submod.config._module.args.name cfg.beforeUserborn then
            "${cfg.settings.decryptedDirForUser}/${submod.config.name}"
          else
            "${cfg.settings.decryptedDir}/${submod.config.name}";
        defaultText = literalExpression ''
          "''${cfg.settings.decryptedDir}/''${config.name}"
        '';
        description = ''
          Path where the decrypted secret is installed.
        '';
      };
      mode = mkOption {
        type = types.str;
        default = "0400";
        description = ''
          Permissions mode of the decrypted secret in a format understood by chmod.
        '';
      };
      owner = mkOption {
        type = types.str;
        default = "root";
        description = ''
          User of the decrypted secret.
        '';
      };
      group = mkOption {
        type = types.str;
        default = users.${submod.config.owner}.group or "root";
        defaultText = literalExpression ''
          users.''${config.owner}.group or "root"
        '';
        description = ''
          Group of the decrypted secret.
        '';
      };
      insert = mkOption {
        type = types.attrsOf (
          lib.types.submodule (submod: {
            options = {
              order = mkOption {
                type = types.ints.u32;
                default = 0;
                defaultText = literalExpression ''
                  0
                '';
                description = ''
                  Unique in `insert` section.
                  Integer for confirming the insertion order.
                  Recommended to set explicity since nix doesn't
                  have ordered attrset.
                '';
              };
              content = mkOption {
                type = types.str;
                description = ''
                  Text to insert.
                '';
              };
              _id = mkOption {
                type =
                  types.addCheck types.str (
                    s:
                    let
                      len = builtins.stringLength s;
                      hexChars = map toString (lib.range 0 9) ++ [
                        "a"
                        "b"
                        "c"
                        "d"
                        "e"
                        "f"
                      ];
                      chars = lib.stringToCharacters s;
                      validateHexChars = lib.all (i: lib.elem i hexChars) chars;
                    in
                    (len == 64) && validateHexChars
                  )
                  // {
                    description = "${types.str.description} (with check: 32 bytes hex lowercase string)";
                  };
                default = submod.config._module.args.name;
                readOnly = true;
              };
            };
          })
        );
        default = { };
        defaultText = literalExpression ''
          { }
        '';
        description = ''
          Inserting plain text to secret.
          See <https://github.com/milieuim/vaultix/issues/12>.
        '';
      };
    };
  });
}
