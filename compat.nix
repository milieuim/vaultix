# This is for compatibility of (Pure Nix without flake-parts framework) users
# This configuration method lacks of option type check, which based on flake-parts module system.
{
  withSystem,
  inputs,
  self,
  ...
}:
{
  flake.configure =
    {
      nodes,
      cache ? "./secrets/cache",
      defaultSecretDirectory ? "./secrets",
      identity,
      extraRecipients ? [ ],
      extraPackages ? [ ],
      pinentryPackage ? null,
      systems ? [
        "x86_64-linux"
        "aarch64-linux"
        "riscv64-linux"
      ],
    }:
    let
      inherit (inputs.nixpkgs) lib;
    in
    {
      # for nixosSystem finding the location with `self.vaultix.*`
      inherit cache defaultSecretDirectory;

      app = lib.genAttrs systems (
        system:
        lib.genAttrs
          [
            "renc"
            "edit"
          ]
          (
            app:
            import ./apps/${app}.nix {
              inherit
                nodes
                identity
                extraRecipients
                extraPackages
                pinentryPackage
                cache
                lib
                ;
              inherit (withSystem system ({ pkgs, ... }: pkgs))
                pkgs
                ;
              package = self.packages.${system}.default;
            }
          )
      );
    };
}
