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
                description = "absolute path string outside of flake repo, or relative path string inside flake repo.";
              };
              default = "/tmp/vaultix.\"$UID\"";
              defaultText = lib.literalExpression "/tmp/vaultix.\"$UID\"";
              description = ''
                `path str` that relative to flake root, used for storing host public key
                re-encrypted secrets. If this is not set or is absolute path string,
                prefetch mode will be automatically enabled.

                Default is the path under /tmp. Could be bash expression resulting in a single
                string.

                If you need to manage multiple flake repo with vaultix, setting this to
                a unique path per flake or using relative path str will be better.

                Example: "\"\$\{XDG_CACHE_HOME:=$HOME/.cache}/vaultix\""
              '';
            };
            redirFileLocation = mkOption {
              type = types.addCheck types.str (s: (builtins.substring 0 1 s) != "/") // {
                description = "path string relative to flake root, inside flake repo";
              };
              default = ".renc-redir.json";
              defaultText = lib.literalExpression ".renc-redir";
              description = ''
                The file contains nix store path string of re-encrypted host secrets.
                Should be update while effectively running `renc`.
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
              type =
                types.addCheck types.str (
                  s:
                  let
                    inherit (builtins) substring;
                  in
                  substring 0 2 s == "./" || substring 0 1 s != "/" || substring 0 11 s != "/nix/store/"
                )
                // {
                  description = ''
                    relative path string inside flake repo, or
                    absolute path string prefixed with `/nix/store/` (which
                    could be flake input outPath, e.g. `inputs.secrets.outPath`).
                  '';
                };
              default = "./secrets";
              defaultText = lib.literalExpression "./secrets";
              description = ''
                `path str` that relative to flake root, used as default path prefix of
                secret. e.g.
                ```nix
                  defaultSecretDirectory = "./secrets";
                  # or
                  # defaultSecretDirectory = inputs.secrets.outPath;
                ```
                then
                ```nix
                  vaultix.secrets.foo = { };
                ```
                will equivalent to:
                ```nix
                  vaultix.secrets.foo = { file = "./secrets/foo.age"; }; # and other default secret options
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
            autoCommit = mkOption {
              type = types.bool;
              default = true;
              description = ''
                Whether automatically commit after `renc` complete.
              '';
            };
            commitMessage = mkOption {
              type = types.str;
              default = "vaultix: re-encrypt for hosts";
              example = "vaultix: re-encrypt for hosts";
              description = ''
                Commit message for auto committing after renc complete.
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
