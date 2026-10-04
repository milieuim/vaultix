{
  inputs,
  self,
  ...
}:
{
  perSystem =
    {
      pkgs,
      self',
      ...
    }:
    let
      craneLib = inputs.crane.mkLib pkgs;
      src = craneLib.cleanCargoSource self;

      pinentryMock = pkgs.writeShellScript "vaultix-test-pinentry" ''
        printf 'OK\n'
        while IFS= read -r command; do
          case "$command" in
            GETPIN)
              printf 'GETPIN\n' >> "$PINENTRY_LOG"
              printf 'D %s\nOK\n' "$TEST_PASSPHRASE"
              ;;
            BYE)
              printf 'OK\n'
              exit 0
              ;;
            *) printf 'OK\n' ;;
          esac
        done
      '';
      editorMock = pkgs.writeShellScript "vaultix-test-editor" ''
        if [ "$TEST_EDITOR_MODE" = replace ]; then
          printf 'edited secret\n' > "$1"
        fi
      '';

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

        identity-edit-integration =
          pkgs.runCommand "vaultix-identity-edit-integration"
            {
              nativeBuildInputs = [
                self'.packages.default
                pkgs.rage
                pkgs.coreutils
                pkgs.gnugrep
              ];
            }
            ''
              set -euo pipefail
              mkdir -p home tmp
              export HOME="$PWD/home" TMPDIR="$PWD/tmp"
              export PINENTRY_PROGRAM="${pinentryMock}"
              export PINENTRY_LOG="$PWD/pinentry.log"
              export TEST_PASSPHRASE=test-passphrase
              export VISUAL="${editorMock}" EDITOR="${editorMock}"
              export TEST_EDITOR_MODE=noop

              rage-keygen -o identity.txt
              recipient="$(rage-keygen -y identity.txt)"
              rage -p -o identity.age identity.txt
              printf 'original secret\n' > plaintext.txt
              rage -r "$recipient" -o secret.age plaintext.txt
              cp secret.age original.age

              : > "$PINENTRY_LOG"
              vaultix edit -i identity.txt secret.age
              cmp secret.age original.age
              test ! -s "$PINENTRY_LOG"

              export TEST_EDITOR_MODE=replace
              vaultix edit -i identity.age secret.age
              test -s "$PINENTRY_LOG"
              rage -d -i identity.txt -o decrypted.txt secret.age
              printf 'edited secret\n' > expected.txt
              cmp decrypted.txt expected.txt

              cp secret.age before-failure.age
              export TEST_PASSPHRASE=wrong-passphrase
              if vaultix edit -i identity.age secret.age 2> error.log; then
                echo 'wrong passphrase unexpectedly succeeded' >&2
                exit 1
              fi
              grep -q 'decrypt identity file' error.log
              cmp secret.age before-failure.age

              touch "$out"
            '';
      };
    };
}
