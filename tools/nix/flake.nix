{
  description = "Nix workspace tooling for Ockam projects";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = inputs @ {flake-parts, ...}:
    flake-parts.lib.mkFlake {inherit inputs;} {
      imports = [./parts/all.nix];
      systems = ["aarch64-darwin" "aarch64-linux" "x86_64-darwin" "x86_64-linux"];

      # see /tools/docker/builder/Dockerfile
      # 24.1.7 stipulated by Dockerfile does not build successfully with current nixpkgs input
      #ockam.elixir.erlangVersion = "26.2.3";
      #ockam.elixir.version = "1.16.2";
      ockam.rust.suggestedCargoPlugins = false;
      ockam.rust.rustAnalyzer = false;
    };
}
