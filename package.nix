{
  shortRev,
  craneLib,
  rustPlatform,
}:
let
  commonArgs = {
    src = craneLib.cleanCargoSource ./.;
    nativeBuildInputs = [
      rustPlatform.bindgenHook
    ];
    strictDeps = true;
  };
in
craneLib.buildPackage (
  commonArgs
  // {
    version = "${(craneLib.crateNameFromCargoToml { cargoToml = ./Cargo.toml; }).version}+${shortRev}";

    cargoVendorDir = craneLib.vendorCargoDeps { cargoLock = ./Cargo.lock; };
    cargoArtifacts = craneLib.buildDepsOnly commonArgs;

    # next-test
    doCheck = false;
    meta.mainProgram = "vaultix";
  }
)
