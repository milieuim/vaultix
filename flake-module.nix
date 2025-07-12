vaultixFlake:
{
  lib,
  self,
  config,
  flake-parts-lib,
  ...
}:
let
  inherit (lib)
    mkOption
    mkPackageOption
    types
    ;

in
{
  options = {
    flake = flake-parts-lib.mkSubmoduleOptions {
      vaultix = mkOption {
        type = types.submodule (submod: {
          options = {
            cache = mkOption {
              type = types.str // {
                description = "path string relative to flake root, or absolute path string.";
              };
              default = "/tmp/vaultix.\"$UID\"";
              defaultText = lib.literalExpression "/tmp/vaultix.\"$UID\"";
              description = ''
                `path str` that relative to flake root, used for storing host public key
                re-encrypted secrets. If this is not set or is absolute path string,
                prefetch mode will be automatically enabled.

                Default is the path under /tmp. Could be bash expression resulting in a single
                string.

                example: "\"\$\{XDG_CACHE_HOME:=$HOME/.cache}/vaultix\""
              '';
            };
            storageLocation = mkOption {
              type = types.nullOr (
                types.addCheck types.str (s: (builtins.substring 0 1 s) == ".")
                // {
                  description = "path string relative to flake root";
                }
              );
              default = null;
              defaultText = lib.literalExpression "null";
              description = ''
                null or `path str` that relative to flake root, used for storing host public key
                re-encrypted secrets. If this is not `null`, host re-encrypted secret will be
                stored in your configuration repo.
              '';
            };
            nodes = mkOption {
              type = types.lazyAttrsOf types.unspecified;
              default = self.nixosConfigurations;
              defaultText = lib.literalExpression "self.nixosConfigurations";
              description = ''
                nixos systems that vaultix to manage.
              '';
            };
            identity = mkOption {
              type =
                with types;
                let
                  identityPathType = coercedTo path toString str;
                in
                nullOr identityPathType;
              default = null;
              example = ./password-encrypted-identity.pub;
              description = ''
                `Age identity file`.
                Able to use yubikey, see <https://github.com/str4d/age-plugin-yubikey>.
                Supports age native secrets (recommend protected with passphrase)
              '';
            };
            defaultSecretDirectory = mkOption {
              type = types.addCheck types.str (s: (builtins.substring 0 1 s) == ".") // {
                description = "path string relative to flake root";
              };
              default = "./secrets";
              defaultText = lib.literalExpression "./secrets";
              description = ''
                `path str` that relative to flake root, used as default path prefix of
                secret. e.g.
                ```nix
                  defaultSecretDirectory = "./secrets";
                ```
                then
                ```nix
                  vaultix.secrets.foo = { };
                ```
                will equivalent to:
                ```nix
                  vaultix.secrets.foo = { file = "./secrets/foo.age"; };
                ```
              '';
            };
            extraRecipients = mkOption {
              type = with types; listOf str;
              default = [ ];
              example = [
                "age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqs3290gq"
              ];
              description = ''
                Recipients used for backup. Any of identity of them will able
                to decrypt all secrets.
              '';
            };
            extraPackages = mkOption {
              type = with types; listOf package;
              default = [ ];
              example = lib.literalExpression "[ pkgs.age-plugin-yubikey ]";
              description = ''
                Set of extra packages like age plugins to be added in edit/renc's path.
              '';
            };
            pinentryPackage = mkPackageOption config.vaultix.pkgs "pinentry-qt" {
              nullable = true;
              default = null;
              extraDescription = ''
                Which pinentry interface to use. If not `null`, the path to the mainProgram
                as defined in the package’s meta attributes will be set to PINENTRY_PROGRAM
                environment variable picked up by edit/renc command.
              '';
            };
            app = mkOption {
              type = types.lazyAttrsOf (types.lazyAttrsOf types.package);
              default = lib.mapAttrs (
                system: config':
                lib.genAttrs
                  [
                    "renc"
                    "edit"
                  ]
                  (
                    app:
                    import ./apps/${app}.nix {
                      inherit (submod.config)
                        nodes
                        identity
                        extraRecipients
                        cache
                        extraPackages
                        pinentryPackage
                        ;
                      inherit (config'.vaultix) pkgs;
                      inherit lib;
                      package = vaultixFlake.packages.${system}.default;
                    }
                  )
              ) config.allSystems;
              readOnly = true;
              defaultText = "Auto generate by flake module";
              description = ''
                vaultix apps that auto generate by its flake module.
                Run manually with `nix run .#vaultix.app.$system.<app-name>`
              '';
            };
          };
        });
        default = { };
        description = ''
          A single-admin secret manage scheme for nixos, with support of templates and
          agenix-like secret configuration layout.
        '';
      };
    };

    perSystem = flake-parts-lib.mkPerSystemOption (
      {
        lib,
        pkgs,
        ...
      }:
      {
        options.vaultix = {
          pkgs = mkOption {
            type = types.unspecified;
            default = pkgs;
            defaultText = lib.literalExpression "pkgs";
            description = ''
              pkgs that passed into vaultix apps.
            '';
          };
        };
      }
    );
  };
  _file = __curPos.file;
}
