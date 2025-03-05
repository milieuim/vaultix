## flakeModule Options

> [!NOTE]
> If you don't like flake-parts, you could skip to another choice without flake-level option type check: [pure nix](./pure-nix-config.md)


This is a flake module configuration, it should be written in your flake top-level or in flake module.

You could find the full definition [here](https://github.com/milieuim/vaultix/blob/main/flake-module.nix)

---

First of all, import the vaultix flake module, and add basic vaultix configs:

```nix
#...

flake-parts.lib.mkFlake { inherit inputs; } (
  { withSystem, ... }:
  {

    # import the flake module
    imports = [ inputs.vaultix.flakeModules.default ];

    flake = {

      # add vaultix flake-level config
      vaultix = {
        # extraRecipients = [ ];            # default, optional
        # cache = "./secrets/cache";        # default, optional
        # nodes = self.nixosConfigurations; # default, optional
        identity = "/somewhere/age-yubikey-identity-deadbeef.txt";
      };
    };
  });
# ...
```

### nodes

+ type: `typeOf nixosConfigurations`

NixOS systems that allow vaultix to manage. Generally pass `self.nixosConfigurations` will work, if you're using framework like `colmena` that produced unstandard system outputs, you need manually conversion, there always some way. For example, for `colmena`:

```nix
nodes = inherit ((colmena.lib.makeHive self.colmena).introspect (x: x)) nodes;
```


### identity

+ type: `string or path`

Age identity file.

Supports age native secrets (recommend protected with passphrase), this could be a:

+ **string (Recommend)**, of **absolute path** to your local age identity. Thus it can avoid loading identity to nix store.

+ **path**, relative to your age identity in your configuration repository. Note that writing path directly will copy your private key into nix store, with **Global READABLE**.

> [!CAUTION]  
> Writing **path** directly (without `"`) will copy your private key into local nix store, with **Global READABLE**. Set path is safe **only** while your private key cannot be directly accessed, such as storing in yubikey or complex passphrase protected.


This is *the identity* that could decrypt all of your secret, take care of it.

> Every `path` type variable in your nix configuration will load file to nix store, eventually shows as string of absolute path to nix store.

example:

```
"/somewhere/age-yubikey-identity-7d5d5540.txt.pub" # note that is string,
                                                   # or your eval will be impure.
./age-yubikey-identity-7d5d5540.txt.pub
"/somewhere/age-private-key"
```

The [Yubikey PIV](https://developers.yubico.com/yubico-piv-tool/YubiKey_PIV_introduction.html) identity with plugin provided better security, but the decryption speed (at re-encryption and edit stage) will depend on your yubikey device.

Since it inherited great compatibility of `age`, you could use [yubikey](https://github.com/str4d/age-plugin-yubikey). Feel free to test other plugins like [age tpm](https://github.com/Foxboron/age-plugin-tpm). 



### extraRecipients

+ type: `list of string`

Age recipients that used as backup keys. Any of them can decrypt all secrets, just like the identity, making them equally critical as [identity](#identity).

This option only takes effect after you finish [editing](/vaultix/nix-apps.html#edit) the secret file.
In other words, changes to this value will not dynamically propagate to existing secrets.
A single-line command to update all secrets globally with this option is currently unsupported

### cache

**String** of path that **relative** to flake root, used for storing host public key
re-encrypted secrets. It's default `./secrets/cache`.

> [!TIP]  
> This directory was automatic created at specified location after first [renc](/vaultix/nix-apps.html#renc), and it should be added to git before deploying.


---

In this way your configuration will looks like:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    vaultix.url = "github:milieuim/vaultix";
  };
  outputs =
    inputs@{
      flake-parts,
      vaultix,
      self,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } ({ ... }:
    {
      imports = [ inputs.vaultix.flakeModules.default ];

      flake = {
        vaultix = {
          nodes = self.nixosConfigurations;
          identity = "/somewhere/some";
          cache = "./secrets/cache";
        };
        nixosConfigurations = {
          tester = withSystem "x86_64-linux" ({system,...}:
            with inputs.nixpkgs;
            lib.nixosSystem {
              inherit system;
              specialArgs = {
                inherit self; # or inputs
              };
              modules = [
                ./configuration.nix
              ];
            }
          );
        };
      };
    });
}
```
