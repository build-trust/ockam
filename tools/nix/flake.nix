{
  description = "Nix workspace tooling for Ockam projects";

  inputs = {
    beam-flakes.url = "github:shanesveller/nix-beam-flakes";
    beam-flakes.inputs.flake-parts.follows = "flake-parts";
    beam-flakes.inputs.nixpkgs.follows = "nixpkgs";
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = inputs @ {flake-parts, ...}:
    flake-parts.lib.mkFlake {inherit inputs;} {
      imports = [inputs.beam-flakes.flakeModule ./parts/all.nix];
      systems = ["aarch64-darwin" "aarch64-linux" "x86_64-darwin" "x86_64-linux"];

      # see /tools/docker/builder/Dockerfile
      # 24.1.7 stipulated by Dockerfile does not build successfully with current nixpkgs input
      ockam.elixir.erlangVersion = "24.3.4.10";
      ockam.elixir.version = "1.13.0";
      ockam.rust.suggestedCargoPlugins = true;
    };
}
