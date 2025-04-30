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
              type = types.addCheck types.str (s: (builtins.substring 0 1 s) == ".") // {
                description = "path string relative to flake root";
              };
              default = "./secrets/cache";
              defaultText = lib.literalExpression "./secrets/cache";
              description = ''
                `path str` that relative to flake root, used for storing host public key
                re-encrypted secrets.
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
