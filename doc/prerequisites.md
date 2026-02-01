# Prerequisites

+ use `flake`
+ `nix-command` and `flake` [experimental feature](https://nix.dev/manual/nix/2.18/contributing/experimental-features) enabled.
+ `inputs` or `self` as one of `specialArgs` for `nixosSystem`
+ `systemd.sysusers` or `services.userborn` option enabled (means you need NixOS 24.11 or newer)

---

> enable `nix-command` `flakes` features

Which almost user done.

Vaultix depends on flake and nix apps perform basic function.

```nix
nix.settings = {
  experimental-features = [
    "nix-command"
    "flakes"
  ];
}
```

---

> `inputs` or `self` as one of `specialArgs` for `nixosSystem`

For passing top-level flake arguments to nixos module.

This requirement may change in the future, with backward compatibility. Looking forward for a better implementation in nixpkgs that more gracefully to do so.

e.g.

```nix
{
  description = "An Example";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    vaultix.url = "github:milieuim/vaultix";
  };

  outputs = { self, nixpkgs, ... }@inputs: {
    nixosConfigurations.my-nixos = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";

      ######################################
      specialArgs = {
        inherit self; # or inputs. You can inherit both as well.
      };
      ######################################

      modules = [
        inputs.vaultix.nixosModules.default
        ./configuration.nix
      ];
      # ...
    };
    # vaultix = ...
  };
}
```

---

> enable `systemd.sysusers` or `services.userborn`

`sysusers` was comes with [Perlless Activation](https://github.com/NixOS/nixpkgs/pull/270727).

`userborn` was introduced in [Aug 30 2024](https://github.com/NixOS/nixpkgs/pull/332719)

Both available in NixOS 24.11 or newer.

Vaultix using systemd instead of old perl script for activating on system startup or switch.
