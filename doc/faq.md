# Frequent Asked Questions

**Q.** Rebooting and unit failed with could not found ssh private key, but it indeed just there.

**A.** Check if using `root on tmpfs`, and modify [hostKeys](https://milieuim.github.io/vaultix/nixos-option.html#hostkeys) path to absolute path string which your REAL private key located (not bind mounted or symlinked etc.). You could also choose setting `needForBoot` for your persist mountpoint. This could also fix similar issue happened with agenix and sops-nix.

---

**Q.** Why another secret management solution for NixOS? 

**A.** Because I don't like Bash, which most solutions rely on. Plus, many lack templating features, and **sops-nix** feels too bloated for my needs.

---

**Q.** What if I don't enable sshd and had no host ssh key previous generated?
**A.** just manually generate it and place in where `services.openssh.hostKeys` default value says.
