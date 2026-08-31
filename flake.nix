{
  description = "Vaultix";

  inputs = {
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    pre-commit-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs = {
        flake-compat.follows = "";
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  outputs =
    inputs@{
      flake-parts,
      self,
      crane,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } (
      {
        flake-parts-lib,
        withSystem,
        ...
      }:
      let
        inherit (flake-parts-lib) importApply;
        flakeModules.default = importApply ./flake-module.nix {
          inherit (self) packages;
          inherit withSystem;
        };
      in
      {
        # debug = true;
        partitionedAttrs = {
          checks = "dev";
          nixosConfigurations = "dev";
          vaultix = "dev";
        };
        partitions = {
          dev.extraInputsFlake = ./dev;
          dev.module = _: {
            imports = [
              flakeModules.default
              ./dev/test.nix
              ./dev/checks.nix
            ];
          };
        };

        imports =
          let
            inherit (inputs) flake-parts;
          in
          [
            flake-parts.flakeModules.partitions
            inputs.pre-commit-hooks.flakeModule
            ./compat.nix
          ];
        systems = [
          "x86_64-linux"
          "aarch64-linux"
        ];
        perSystem =
          {
            self',
            pkgs,
            system,
            config,
            ...
          }:
          {
            apps = {
              default = {
                type = "app";
                program = pkgs.lib.getExe self'.packages.default;
              };
            };

            packages =
              let
                p = (self.overlays.default pkgs pkgs);
              in
              p
              // {
                default = p.vaultix;
              };

            formatter = pkgs.nixfmt-tree;

            pre-commit = {
              check.enable = true;
              settings.hooks = {
                nixfmt.enable = true;
                # clippy = {
                #   enable = true;
                #   packageOverrides.cargo = pkgs.cargo;
                #   packageOverrides.clippy = pkgs.clippy;
                #   # some hooks provide settings
                #   settings.allFeatures = true;
                # };
              };
            };

            devShells.default = (inputs.crane.mkLib pkgs).devShell {
              shellHook = config.pre-commit.installationScript;
              inputsFrom = [
                self'.packages.default
              ];
              packages = with pkgs; [
                just
                nushell
                cargo-fuzz
                statix
                typos
                act
              ];
            };
          };

        flake = {
          inherit flakeModules;
          nixosModules = rec {
            default =
              { pkgs, ... }:
              {
                imports = [ ./module ];
                vaultix.package = (self.overlays.default pkgs pkgs).vaultix;
              };
            vaultix = default;
          };

          overlays.default = (
            final: prev: {
              vaultix = final.callPackage ./package.nix {
                shortRev = self.shortRev or "dirty";
                craneLib = inputs.crane.mkLib final;
              };
            }
          );
        };
      }
    );
}
