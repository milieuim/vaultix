{
  withSystem,
  self,
  inputs,
  ...
}:
{
  flake = {
    vaultix = {
      # minimal works configuration
      nodes = self.nixosConfigurations;
      identity = "/home/riro/Src/vaultix/dev/test_key/ed25519_ssh_key_with_passphrase_123456";

      cache = "./dev/secrets/cache"; # relative to the flake root.
    };
    nixosConfigurations = {
      tester = withSystem "x86_64-linux" (
        {
          system,
          ...
        }:
        with inputs.nixpkgs;
        lib.nixosSystem (
          lib.warn
            ''
              THIS SYSTEM IS ONLY FOR TESTING,
              If this msg appears in production
              there MUST be something wrong,
              please stop operation immediately
              then check the code.
            ''
            {
              inherit system;
              specialArgs = {
                inherit
                  self # Required
                  inputs
                  ;
              };
              modules = [
                self.nixosModules.vaultix

                (
                  { config, ... }:
                  {
                    services.userborn.enable = true; # or systemd.sysuser, required

                    vaultix = {
                      # DON'T COPY THIS
                      settings.hostPubkey = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEu8luSFCts3g367nlKBrxMdLyOy4Awfo5Rb397ef2AR";

                      beforeUserborn = [ "test-secret-2_before" ];
                      secrets = {

                        # secret example
                        test-secret-1 = {
                          file = ./secrets/test.age;
                          mode = "400";
                          owner = "root";
                          group = "users";
                          # path = "/home/1.txt";
                        };
                        test-secret-2_before = {
                          file = ./secrets/test.age;
                          mode = "400";
                          owner = "root";
                          group = "users";
                          # path = "/home/1.txt";
                        };
                        test-secret-3_arb_path = {
                          file = ./secrets/test.age;
                          mode = "400";
                          owner = "root";
                          group = "users";
                          path = "/home/1.txt";
                        };
                        test-secret-insert = {
                          file = ./secrets/ins-sec.age;
                          insert = {
                            "4d060ab79d5f0827289e353d55e14273acb5b61bc553b1435b5729fea51e6ff7" = {
                              order = 0;
                              content = "Alice was beginning to get very tired of sitting by her sister on the bank";
                            };
                            "9e924b9a440a09ccb97d27a3bd4166a1ad8c10af65857606abdffe41940f129d" = {
                              order = 1;
                              content = "but it had no pictures or conversations in it";
                            };
                          };
                        };
                      };

                      # template example
                      templates.template-test = {
                        name = "template.txt";
                        content = ''
                          Down the Rabbit-Hole\n${config.vaultix.placeholder.test-secret-1}
                        '';
                        path = "/var/template.txt";
                      };

                    };

                    # for vm testing log
                    systemd.services.vaultix-activate.serviceConfig.Environment = [ "RUST_LOG=trace" ];
                  }
                )

                ./configuration.nix
                (
                  { config, pkgs, ... }:
                  {
                    disko.tests = {
                      extraChecks = ''
                        machine.succeed("test -e /run/vaultix.d/normal")
                        machine.succeed("test -e /run/vaultix.d/early")
                        machine.succeed("test -e /run/vaultix.d/normal/0")
                        machine.succeed("test -e /run/vaultix.d/early/0")
                        machine.succeed("test -e ${config.vaultix.secrets.test-secret-1.path}")
                        machine.succeed("test -e ${config.vaultix.secrets.test-secret-2_before.path}")
                        machine.succeed("test -e ${config.vaultix.secrets.test-secret-3_arb_path.path}")
                        machine.succeed("test -e ${config.vaultix.templates.template-test.path}")
                        machine.succeed("test -e ${config.vaultix.secrets.test-secret-insert.path}")
                        machine.succeed("md5sum -c ${pkgs.writeText "checksum-list" ''
                          9ccb444ead3f065d8322cee5a6838e9b ${config.vaultix.secrets.test-secret-1.path}
                          9ccb444ead3f065d8322cee5a6838e9b ${config.vaultix.secrets.test-secret-2_before.path}
                          e3cbe115932231e2061cf09b9daffac6 ${config.vaultix.templates.template-test.path}
                          9ccb444ead3f065d8322cee5a6838e9b ${config.vaultix.secrets.test-secret-insert.path}
                        ''}")
                      '';
                    };
                  }
                )
              ];
            }
        )
      );
    };
  };
}
