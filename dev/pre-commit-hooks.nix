{ ... }:
{
  perSystem =
    { pkgs, ... }:
    {
      pre-commit = {
        check.enable = true;
        # TODO: the check currently unavaliable
        settings.hooks = {
          nixfmt-rfc-style.enable = true;
          clippy.enable = true;
          clippy.packageOverrides.cargo = pkgs.cargo;
          clippy.packageOverrides.clippy = pkgs.clippy;
          # some hooks provide settings
          clippy.settings.allFeatures = true;
        };
      };
    };
}
