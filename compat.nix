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
      identity,
      extraRecipients ? [ ],
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
      # for nixosSystem finding the cache location
      inherit cache;

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
