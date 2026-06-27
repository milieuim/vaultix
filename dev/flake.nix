{
  description = "partition. tests and dev cfg for vaultix";

  inputs = {
    # for create system for testing
    disko = {
      url = "github:nix-community/disko";
    };

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = _: { };
}
