# Settings
Literally.


<div id="dd"></div>

### decryptedDir

+ type: `string of absolute path`
+ default: `/run/vaultix`

Folder where secrets are symlinked to.

### decryptedDirForUser

+ type: `string of absolute path`
+ default: `/run/vaultix-for-user`

Same as above, but for secrets and templates that required by user, which means needs to be initialize before user born.


<div id="dmp"></div>

### decryptedMountPoint

+ type: `string of absolute path`
+ default: `/run/vaultix.d`

Path str with no trailing slash

Where secrets are created before they are symlinked to `vaultix.settings.decryptedDir`

Vaultix use this machenism to implement atomic manage, like other secret managing schemes.

It decrypting secrets into this directory, with generation number like `/run/vaultix.d/1`, then symlink it to `decryptedDir`.

### hostKeys

+ type: `{ path: str, type: str }`
+ default: `config.services.openssh.hostKeys`

This generally has no need to manually change, unless you know clearly what you're doing.

Ed25519 host private ssh key (identity) path that used for decrypting secrets while deploying.

format:

```nix
[
  {
    path = "/path/to/ssh_host_ed25519_key";
    type = "ed25519";
  }
]
```

### hostPubkey

+ type: `(string of pubkey) or (path of pubkey file)`

example:

```nix
hostPubkey = "ssh-ed25519 AAAAC3Nz....."
# or
hostPubkey = ./ssh_host_ed25519_key.pub # no one like this i think
```

ssh public key of the hostKey. This is different from every host, since each generates host key while initial booting.

Get this of remote machine by: `ssh-keyscan ip`. It supports `ed25519` type.

You could find it in `/etc/ssh/` next to host ssh private key, with `.pub` suffix.

This could be either literal string or path, the previous one is more recommended.

