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
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs =
    inputs@{
      flake-parts,
      self,
      crane,
      advisory-db,
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
            ];
          };
        };

        imports =
          let
            inherit (inputs) flake-parts;
          in
          [
            flake-parts.flakeModules.easyOverlay
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
          let
            craneLib = crane.mkLib pkgs;
            inherit (craneLib) buildPackage;
            src = craneLib.cleanCargoSource ./.;

            commonArgs = {
              inherit src;
              nativeBuildInputs = [
                pkgs.rustPlatform.bindgenHook
              ];
              strictDeps = true;
            };
            cargoVendorDir = craneLib.vendorCargoDeps { cargoLock = ./Cargo.lock; };
            cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          in
          {
            _module.args.pkgs = import inputs.nixpkgs {
              inherit system;
              overlays = [
                inputs.self.overlays.default
              ];
            };
            apps = {
              default = {
                type = "app";
                program = pkgs.lib.getExe self'.packages.default;
              };
            };

            packages = rec {
              default = buildPackage (
                commonArgs
                // {
                  version =
                    (craneLib.crateNameFromCargoToml { cargoToml = ./Cargo.toml; }).version
                    + "+"
                    + self.shortRev or "dirty";
                  inherit cargoArtifacts cargoVendorDir;

                  # next-test
                  doCheck = false;
                  meta.mainProgram = "vaultix";
                }
              );
              vaultix = default;
            };
            overlayAttrs = config.packages;

            formatter = pkgs.nixfmt-tree;

            pre-commit = {
              check.enable = true;
              settings.hooks = {
                nixfmt-rfc-style.enable = true;
                # clippy = {
                #   enable = true;
                #   packageOverrides.cargo = pkgs.cargo;
                #   packageOverrides.clippy = pkgs.clippy;
                #   # some hooks provide settings
                #   settings.allFeatures = true;
                # };
              };
            };

            devShells.default = craneLib.devShell {
              shellHook = config.pre-commit.installationScript;
              inputsFrom = [
                pkgs.vaultix
              ];
              buildInputs = with pkgs; [
                just
                nushell
                cargo-fuzz
                statix
                typos
                act
              ];
            };

            checks = {
              # Audit dependencies
              crate-audit = craneLib.cargoAudit {
                inherit src advisory-db cargoVendorDir;
                # RUSTSEC-2023-0071: Marvin Attack: potential key recovery through timing sidechannels
                cargoAuditExtraArgs = "--ignore RUSTSEC-2023-0071";
              };

              crate-nextest = craneLib.cargoNextest (
                commonArgs
                // {
                  inherit cargoArtifacts cargoVendorDir;
                  partitions = 1;
                  partitionType = "count";
                  cargoNextestPartitionsExtraArgs = "--no-tests=pass";
                }
              );
            };
          };
        flake = {
          inherit flakeModules;
          nixosModules = rec {
            default =
              { pkgs, ... }:
              {
                imports = [ ./module ];
                vaultix.package = withSystem pkgs.stdenv.hostPlatform.system (
                  { config, ... }: config.packages.vaultix
                );
              };
            vaultix = default;
          };
        };
      }
    );
}
