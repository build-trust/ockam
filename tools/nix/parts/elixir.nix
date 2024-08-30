{
  config,
  lib,
  ...
}: let
    inherit (lib) mkEnableOption;
    cfg = config.ockam.elixir;
  in {
    options = {
      ockam.elixir = {
        languageServer = mkEnableOption "elixir-ls" // {default = true;};
      };
    };

    config = {
      perSystem = {
          config,
          lib,
          pkgs,
          ...
        }: {
          devShells.elixir = pkgs.mkShell {
            packages = with pkgs; [
              erlang_24
              elixir_1_13
            ] ++ lib.optional cfg.languageServer elixir-ls;

            # ockam_vault_software uses a Rust NIF
            inputsFrom = with config.devShells; [rust tooling];
            # support pkgconfig without duplication of effort
            inherit (config.devShells.rust) nativeBuildInputs shellHook;

            inherit (config.devShells.rust) OCKAM_DISABLE_UPGRADE_CHECK RUSTFLAGS RUST_SRC_PATH LIBCLANG_PATH;
            inherit (config.devShells.tooling) BATS_LIB;
          };
        };
    };
}
