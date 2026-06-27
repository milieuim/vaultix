{
  inputs,
  self,
  ...
}:
{
  perSystem =
    {
      pkgs,
      ...
    }:
    let
      craneLib = inputs.crane.mkLib pkgs;
      src = craneLib.cleanCargoSource self;

      commonArgs = {
        inherit src;
        nativeBuildInputs = [
          pkgs.rustPlatform.bindgenHook
        ];
        strictDeps = true;
      };
      cargoVendorDir = craneLib.vendorCargoDeps { cargoLock = self + "/Cargo.lock"; };
      cargoArtifacts = craneLib.buildDepsOnly commonArgs;
    in
    {
      checks = {
        # Audit dependencies
        crate-audit = craneLib.cargoAudit {
          inherit src cargoVendorDir;
          advisory-db = inputs.advisory-db;
          # RUSTSEC-2023-0071: Marvin Attack: potential key recovery through timing sidechannels
          # https://github.com/RustCrypto/RSA/issues/626
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
}
