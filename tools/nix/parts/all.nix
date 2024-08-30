{
  imports = [
    ./overlays.nix
    ./elixir.nix
    ./examples.nix
    ./rust.nix
    ./tooling.nix
    ./typescript.nix
  ];

  perSystem = {
    config,
    pkgs,
    ...
  }: {
    devShells.default = pkgs.mkShell {
      inputsFrom = with config.devShells; [elixir rust tooling typescript];

      inherit (config.devShells.rust) nativeBuildInputs CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER DYLD_FALLBACK_LIBRARY_PATH OCKAM_DISABLE_UPGRADE_CHECK RUSTFLAGS RUST_SRC_PATH CARGO_INCREMENTAL LIBCLANG_PATH;
      inherit (config.devShells.tooling) BATS_LIB;

      shellHook = ''
        ${config.devShells.rust.shellHook or ""}

        [ -z "$HOME" ] && export HOME=~
      '';
    };
  };
}
