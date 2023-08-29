{
  imports = [
    ./overlays.nix
    ./elixir.nix
    ./examples.nix
    ./rust.nix
    ./tauri.nix
    ./tooling.nix
    ./typescript.nix
  ];

  perSystem = {
    config,
    pkgs,
    ...
  }: {
    devShells.default = pkgs.mkShell {
      inputsFrom = with config.devShells; [elixir rust tauri tooling typescript];

      inherit (config.devShells.elixir) ASDF_ELIXIR_VERSION ASDF_ERLANG_VERSION;
      inherit (config.devShells.rust) nativeBuildInputs CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER DYLD_FALLBACK_LIBRARY_PATH OCKAM_DISABLE_UPGRADE_CHECK RUSTFLAGS RUST_SRC_PATH CARGO_INCREMENTAL;
      inherit (config.devShells.tooling) BATS_LIB;

      # TODO: Move HOME override to wrapper script or other localized definition
      shellHook = ''
        ${config.devShells.rust.shellHook or ""}

        export HOME=$PWD/.home
      '';
    };
  };
}
