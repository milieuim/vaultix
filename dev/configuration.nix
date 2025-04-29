# NOTICE: All configuration in this file is just for testing
# Not part of vaultix deploy requirements
{
  inputs,
  modulesPath,
  pkgs,
  lib,
  ...
}:
{
  imports = [
    inputs.disko.nixosModules.disko
    ./UEFI.nix
    (modulesPath + "/profiles/qemu-guest.nix")
    # (modulesPath + "/profiles/perlless.nix") :(

    # reduce size. see https://sidhion.com/blog/posts/nixos_server_issues
    (
      { lib, ... }:
      {
        disabledModules = [ "security/wrappers/default.nix" ];

        options.security = {
          wrappers = lib.mkOption {
            type = lib.types.attrs;
            default = { };
          };
          wrapperDir = lib.mkOption {
            type = lib.types.path;
            default = "/run/wrappers/bin";
          };
        };
        config = {
          # ...
        };
      }
    )
  ];
  # WARN: This is just for testing and demonstrating, you SHOULD NOT set like this
  # WARN: This is just for testing and demonstrating, you SHOULD NOT set like this
  # WARN: This is just for testing and demonstrating, you SHOULD NOT set like this
  services.openssh.hostKeys = [
    {
      path = pkgs.writeText "UNSAFE-SSH-PRIVATE-KEY" ''
        -----BEGIN OPENSSH PRIVATE KEY-----
        b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW
        QyNTUxOQAAACBLvJbkhQrbN4N+u55Sga8THS8jsuAMH6OUW9/e3n9gEQAAAJA2pUBZNqVA
        WQAAAAtzc2gtZWQyNTUxOQAAACBLvJbkhQrbN4N+u55Sga8THS8jsuAMH6OUW9/e3n9gEQ
        AAAECY8KpFz3qti09XPK9+gNe1hiBe/KF8tVI+se0e+e1QMEu8luSFCts3g367nlKBrxMd
        LyOy4Awfo5Rb397ef2ARAAAAC2VsZW5Aa2FhbWJsAQI=
        -----END OPENSSH PRIVATE KEY-----
      '';
      type = "ed25519";
    }
  ];

  # eliminate size
  services.lvm.enable = false;
  security.sudo.enable = false;
  users.allowNoPasswordLogin = true;
  documentation.man.enable = lib.mkForce false;

  boot.kernelPackages = pkgs.linuxPackages_latest;

  # users.mutableUsers = false;

  # systemd.network.enable = false;
  # services.resolved.enable = false;
  # networking.networkmanager.enable = false;

  networking.useNetworkd = true;

  networking.hostName = "tester";

  system.stateVersion = "24.05";
}
