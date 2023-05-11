{
  imports = [
    ./overlays.nix
    ./elixir.nix
    ./examples.nix
    ./rust.nix
    ./tooling.nix
  ];

  perSystem = {
    config,
    pkgs,
    ...
  }: {
    devShells.default = pkgs.mkShell {
      inputsFrom = with config.devShells; [elixir rust tooling];

      inherit (config.devShells.elixir) ASDF_ELIXIR_VERSION ASDF_ERLANG_VERSION;
      inherit (config.devShells.rust) nativeBuildInputs shellHook CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER OCKAM_DISABLE_UPGRADE_CHECK RUSTFLAGS RUST_SRC_PATH;
      inherit (config.devShells.tooling) BATS_LIB;
    };
  };
}
