# Pure Nix Configuration


> [!NOTE]
> If you completed setup with flake-parts' [flakeModule](./flake-module.md), you can skip this section and jump to [nixos module setup](nixos-option.md)


The option is identical to [flakeModule](./flake-module.md), but different way to perform nix app producing.


```nix
vaultix = inputs.vaultix.configure {

  # identical with flake-parts way
  nodes = self.nixosConfigurations;
  identity = self + "/age-yubikey-identity-deadbeef.txt.pub";
  extraRecipients = [ ];
  extraPackages = [ ];
  pinentryPackage = null;
  cache = "./secret/.cache";
  # generating `outputs.vaultix.app.${system}.*`
  systems = ["x86_64-linux","aarch64-linux"];
};
```

---

Overview of flake in this configuration:

```nix
{
  description = "An Example";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    vaultix.url = "github:milieuim/vaultix";
  };

  outputs = { self, nixpkgs, vaultix, ... }@inputs: {
    nixosConfigurations.my-nixos = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";

      specialArgs = {
        inherit self;
      };

      modules = [
        inputs.vaultix.nixosModules.default
        ./configuration.nix
      ];
      # ...
    };
    vaultix = vaultix.configure {
    
      # identical with flakeModule way
      nodes = self.nixosConfigurations;
      identity = self + "/age-yubikey-identity-deadbeef.txt.pub";
      extraRecipients = [ ];
      extraPackages = [ ];
      pinentryPackage = null;
      cache = "./secret/.cache";
    };
  };
}
```
