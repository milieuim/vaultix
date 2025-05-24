# Cheat Sheets
Common used workflow with vaultix.

## Generate new identity

```bash
age-keygen # or rage-keygen
```

For whom using Yubikey with `age-plugin-yubikey`, see [configuration](https://github.com/str4d/age-plugin-yubikey?tab=readme-ov-file#configuration)


## Add new secret


### 1. Run edit:

```bash
nix run .#vaultix.app.x86_64-linux.edit -- ./path/to/new-to-add.age
```

### 2. Add a secret to nixos module:

```diff
secrets = {
  #...
+  new-to-add.file = ./path/to/new-to-add.age;
};
```

### 3. Add it to git

### 4. Run renc:


```bash
nix run .#vaultix.app.x86_64-linux.renc
```

### 4. Add all produced stuff to git.



## Modify existed secret


```bash
nix run .#vaultix.app.x86_64-linux.edit -- ./path/to/to-edit.age
```

```bash
nix run .#vaultix.app.x86_64-linux.renc
```

Then add changes to git.

## Remove secret


```diff
secrets = {
  #...
-  new-to-add.file = ./path/to/new-to-add.age;
};
```

```bash
rm ./path/to/new-to-add.age
```

```bash
nix run .#vaultix.app.x86_64-linux.renc
```
