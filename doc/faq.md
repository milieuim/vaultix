# Frequent Asked Questions

**Q.** Rebooting and unit failed with could not found ssh private key, but it indeed just there.

**A.** Check if using `root on tmpfs`, and modify [hostKeys](https://milieuim.github.io/vaultix/nixos-option.html#hostkeys) path to absolute path string which your REAL private key located (not bind mounted or symlinked etc.). You could also choose setting `needForBoot` for your persist mountpoint. This could also fix similar issue happened with agenix and sops-nix.

---

**Q.** Why another secret management solution for NixOS? 

**A.** Because I don't like Bash, which most solutions rely on. Plus, many lack templating features, and **sops-nix** feels too bloated for my needs.

---

**Q.** What if I don't enable sshd and had no host ssh key previous generated?
**A.** just manually generate it and place in where `services.openssh.hostKeys` default value says.

---

**Q.** Nix path lacks a signature by a trusted key while deploying remote target
**A.** if you don't add your user to `trusted-users`, you need to deploy to target with user `root`.

---
**Q.** Why are my uncommitted edits to a secret inside a Git **submodule** not detected by renc when the outer repository is clean?


**A.**

You need to manually introduce a tracked "dirty" state in the outer repository before executing renc.

```
# create a dummy file to dirty the outer tree
touch .nix-dirty

# stage the intent to add this file
git add -N .nix-dirty
```

When evaluating a Nix Flake, Nix optimizes the process by fetching the source tree directly from the Git object database if the outer repository is completely clean. Because it reads from the Git history rather than the physical disk, any local, uncommitted changes inside the submodule's working directory are invisible to Nix. By staging a dummy file in the outer repository, you force Nix into a "dirty tree" fallback mode. In this mode, Nix abandons the pure Git fetch and physically copies your actual working tree into the Nix Store, thereby capturing your latest, uncommitted submodule edits.(Uncertainty Note: While this behavior is widely documented as Issue #13324 in the Nix community, it is uncertain if future releases of Nix will introduce experimental features that natively resolve this dirty-tree boundary issue without requiring manual intervention).Q: I committed the secret changes inside the submodule, but running renc in the outer repository still skips them. Why does this happen?  Concise Solution

See <https://github.com/NixOS/nix/issues/13324>
