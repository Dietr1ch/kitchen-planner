{
  description = "Kitchen Planner";

  inputs = {
    nixpkgs = {
      url = "github:NixOS/nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";

    git-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }@inputs:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # rustManifest = (pkgs.lib.importTOML ./Cargo.toml).package;
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        # rustPlatform = pkgs.makeRustPlatform {
        #   cargo = rustToolchain;
        #   rustc = rustToolchain;
        # };
      in
      {
        devShells = {
          default = import ./shell.nix { inherit pkgs rustToolchain; };
        };

        checks = {
          # TODO: Enable testing support once dependencies stabilise
          # tests = rustPlatform.buildRustPackage {
          #   pname = rustManifest.name;
          #   version = rustManifest.version;
          #   src = ./.;
          #   cargoLock.lockFile = ./Cargo.lock;

          #   nativeBuildInputs = [ pkgs.cargo-nextest ];

          #   buildPhase = "true"; # skip default build
          #   checkPhase = "cargo nextest run";
          #   installPhase = "touch $out";
          # };
          # TODO: Enable pre-commit checks
          # pre-commit-check = inputs.git-hooks.lib.${system}.run {
          #   src = ./.;
          #   hooks = {
          #     nixfmt.enable = true;
          #   };
          # };
        };
      }
    );
}
