set shell := ["nu", "-c"]

pwd := `pwd`

default:
    @just --choose

build-package:
    nix build .

clean-exist-deploy:
    #!/usr/bin/env nu
    sudo umount /run/vaultix.d
    sudo rm -r /run/vaultix.d
    sudo rm -r /run/vaultix
full-test:
    #!/usr/bin/env nu
    cargo test
    just vm-tests
eval-tester:
    nix eval .#nixosConfigurations.tester.config.system.build.toplevel
vm-tests:
    #!/usr/bin/env nu
    nix run github:nix-community/nixos-anywhere -- --flake .#tester --vm-test

# Run GitHub Actions locally using act (supports podman rootless)
act *args:
    #!/usr/bin/env nu
    def main [...args: string] {
        let socket = $"/run/user/(id -u)/podman/podman.sock"
        if ($socket | path exists) {
            $env.DOCKER_HOST = $"unix://($socket)"
            act --container-daemon-socket $socket ...$args
        } else {
            act ...$args
        }
    }
