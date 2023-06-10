{
  description = "Nix workspace tooling for Ockam projects";

  inputs = {
    beam-flakes.url = "github:shanesveller/nix-beam-flakes";
    beam-flakes.inputs.flake-parts.follows = "flake-parts";
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = inputs @ {flake-parts, ...}:
    flake-parts.lib.mkFlake {inherit inputs;} {
      imports = [inputs.beam-flakes.flakeModule ./parts/all.nix];
      systems = ["aarch64-darwin" "x86_64-darwin" "x86_64-linux"];

      ockam.elixir.erlangVersion = "25.3.2";
      ockam.elixir.version = "1.14.5";

      ockam.rust.suggestedCargoPlugins = true;
      ockam.rust.version = "1.69.0";
    };
}
