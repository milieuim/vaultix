{
  description = "Vaultix";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
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
        partitionedAttrs = {
          checks = "dev";
          nixosConfigurations = "dev";
          vaultix = "dev";
        };
        partitions = {
          dev.extraInputsFlake = ./dev;
          dev.module =
            { inputs, ... }:
            {
              imports = [
                inputs.pre-commit-hooks.flakeModule
                flakeModules.default
                ./dev/pre-commit-hooks.nix
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
            target = (pkgs.lib.systems.elaborate system).config;
            mkOverriddenToolchain =
              scale:
              scale.override {
                extensions = [ "rust-src" ];
                targets = [ target ];
              };
            mkCraneLib = toolchain: (crane.mkLib pkgs).overrideToolchain toolchain;
            releaseToolChain = mkOverriddenToolchain pkgs.rust-bin.nightly.latest.minimal;
            releaseCraneLib = mkCraneLib releaseToolChain;
            devCraneLib = mkCraneLib (mkOverriddenToolchain pkgs.rust-bin.nightly.latest.complete);
            inherit (releaseCraneLib) buildPackage;
          in
          {
            _module.args.pkgs = import inputs.nixpkgs {
              inherit system;
              overlays = with inputs; [
                rust-overlay.overlays.default
                self.overlays.default
              ];
            };
            apps = {
              default = {
                type = "app";
                program = pkgs.lib.getExe self'.packages.default;
              };
            };

            packages = rec {
              default = buildPackage rec {
                version =
                  (releaseCraneLib.crateNameFromCargoToml { cargoToml = ./Cargo.toml; }).version
                  + "+"
                  + self.shortRev or "dirty";
                src = releaseCraneLib.cleanCargoSource ./.;
                nativeBuildInputs = [
                  pkgs.rustPlatform.bindgenHook
                ];
                cargoVendorDir = releaseCraneLib.vendorMultipleCargoDeps {
                  inherit (releaseCraneLib.findCargoFiles src) cargoConfigs;
                  cargoLockList = [
                    ./Cargo.lock

                    # Unfortunately this approach requires IFD (import-from-derivation)
                    # otherwise Nix will refuse to read the Cargo.lock from our toolchain
                    # (unless we build with `--impure`).
                    #
                    # Another way around this is to manually copy the rustlib `Cargo.lock`
                    # to the repo and import it with `./path/to/rustlib/Cargo.lock` which
                    # will avoid IFD entirely but will require manually keeping the file
                    # up to date!
                    "${releaseToolChain.passthru.availableComponents.rust-src}/lib/rustlib/src/rust/library/Cargo.lock"
                  ];
                };

                cargoExtraArgs = ''-Z build-std -Z build-std-features="optimize_for_size" --target ${target}'';
                meta.mainProgram = "vaultix";
              };
              vaultix = default;
            };
            overlayAttrs = config.packages;

            formatter = pkgs.nixfmt-rfc-style;

            devShells.default = devCraneLib.devShell {
              shellHook = config.pre-commit.installationScript;
              inputsFrom = [
                pkgs.vaultix
              ];
              buildInputs = with pkgs; [
                just
                nushell
                cargo-fuzz
                mdbook-alerts
                statix
                typos
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
