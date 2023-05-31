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

      inherit (config.devShells.elixir) ASDF_ELIXIR_VERSION ASDF_ERLANG_VERSION;
      inherit (config.devShells.rust) nativeBuildInputs CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER OCKAM_DISABLE_UPGRADE_CHECK RUSTFLAGS RUST_SRC_PATH;
      inherit (config.devShells.tooling) BATS_LIB;

      shellHook = ''
        ${config.devShells.rust.shellHook or ""}
        export DYLD_FALLBACK_LIBRARY_PATH=$(rustc --print sysroot)/lib

        export HOME=$PWD/.home
        export PATH=$PWD/.home/.cargo/bin:$PATH
        cargo install tauri-cli --version "^2.0.0-alpha"
      '';
    };
  };
}
