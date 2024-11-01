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
    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
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
      { flake-parts-lib, withSystem, ... }:
      let
        inherit (flake-parts-lib) importApply;
        flakeModules.default = importApply ./flake-module.nix {
          inherit (self) packages;
          inherit withSystem;
        };
      in
      {
        imports = with inputs; [
          pre-commit-hooks.flakeModule
          flakeModules.default
          ./test
        ];
        systems = [
          "x86_64-linux"
          "aarch64-linux"
        ];
        perSystem =
          {
            # config,
            self',
            # inputs',
            pkgs,
            system,
            ...
          }:
          {
            _module.args.pkgs = import inputs.nixpkgs {
              inherit system;
              overlays = with inputs; [
                rust-overlay.overlays.default
                self.overlays.default
              ];
            };

            vaultix = {
              nodes = self.nixosConfigurations;
            };
            apps = {
              default = {
                type = "app";
                program = pkgs.lib.getExe self'.packages.default;
              };
            };

            packages = rec {
              default =
                let
                  toolchain = pkgs.rust-bin.nightly.latest.minimal;
                  craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
                  inherit (craneLib) buildPackage;
                in
                (buildPackage {
                  src = craneLib.cleanCargoSource ./.;
                  nativeBuildInputs = [
                    pkgs.rustPlatform.bindgenHook
                  ];
                  meta.mainProgram = "vaultix";
                });
              vaultix = default;
            };

            formatter = pkgs.nixfmt-rfc-style;

            devShells.default = pkgs.mkShell {
              inputsFrom = [
                pkgs.vaultix
              ];

              RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
              buildInputs = with pkgs; [
                just
                nushell
              ];
            };

            pre-commit = {
              check.enable = true;
              settings.hooks = {
                nixfmt-rfc-style.enable = true;
              };
            };

          };
        flake = {
          inherit flakeModules;

          overlays.default = final: prev: {
            vaultix = inputs.self.packages.${prev.system}.default;
          };
          nixosModules.default = import ./module self;

        };
      }
    );
}
