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
      identity = "/home/who/unsafe-test";

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
            "THIS SYSTEM IS ONLY FOR TESTING, If this msg appears in production there MUST be something wrong, please stop operation immediately then check the code."
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
                              content = "1st";
                            };
                            "9e924b9a440a09ccb97d27a3bd4166a1ad8c10af65857606abdffe41940f129d" = {
                              order = 1;
                              content = "2nd";
                            };
                          };
                        };
                      };

                      # template example
                      templates.template-test = {
                        name = "template.txt";
                        content = ''
                          for testing vaultix template ${config.vaultix.placeholder.test-secret-1} nya
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
                        machine.succeed("test -e /run/vaultix.d/0")
                        machine.succeed("test -e /run/vaultix.d/1")
                        machine.succeed("test -e ${config.vaultix.secrets.test-secret-1.path}")
                        machine.succeed("test -e ${config.vaultix.secrets.test-secret-2_before.path}") # two generation produced bcz of pre-user unit
                        machine.succeed("test -e ${config.vaultix.secrets.test-secret-3_arb_path.path}")
                        machine.succeed("test -e ${config.vaultix.templates.template-test.path}")
                        machine.succeed("test -e ${config.vaultix.secrets.test-secret-insert.path}")
                        machine.succeed("md5sum -c ${pkgs.writeText "checksum-list" ''
                          6265b22b66502d70d5f004f08238ac3c ${config.vaultix.secrets.test-secret-1.path}
                          6265b22b66502d70d5f004f08238ac3c ${config.vaultix.secrets.test-secret-2_before.path}
                          84b46d85713864d2583f4173c02af215 ${config.vaultix.templates.template-test.path}
                          50793cd107827c2cc96bdf689755ec92 ${config.vaultix.secrets.test-secret-insert.path}
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
