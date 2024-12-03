# Threat Model

The Project based on age, inherited age thread model. See [age spec](https://github.com/C2SP/C2SP/blob/main/age.md).
Here’s a more natural, polished version of your text:

**Vaultix** ensures that your plaintext secrets are never stored in the Nix store with globally readable permissions or written to disk, while also securing them during network transmission.

## About "Harvest Now, Decrypt Later"

The [Harvest Now, Decrypt Later](https://en.wikipedia.org/wiki/Harvest_now,_decrypt_later) strategy involves collecting and storing encrypted files with the aim of decrypting them in the future, potentially using **quantum computers**.

If your configuration is exposed in a public repository, **Vaultix**—like most other NixOS secret management solutions—cannot fully mitigate this risk. For more context, see this [issue](https://github.com/FiloSottile/age/issues/578) and [discussion](https://github.com/FiloSottile/age/discussions/231).

For those concerned about this threat, consider using [age-plugin-sntrup761x25519](https://github.com/keisentraut/age-plugin-sntrup761x25519), which offers post-quantum encryption. This plugin relies on [Rust bindings](https://github.com/rustpq/pqcrypto) for C implementations of cryptographic algorithms from the [NIST Post-Quantum Cryptography competition](https://csrc.nist.gov/projects/post-quantum-cryptography). However, it’s important to note that this solution has not undergone extensive security review. 

