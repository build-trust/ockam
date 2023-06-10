{
  config,
  inputs,
  lib,
  ...
}: let
  inherit (lib) mkEnableOption mkOption types;
  cfg = config.ockam.elixir;
in {
  options = {
    ockam.elixir = {
      erlangVersion = mkOption {
        type = types.nullOr types.string;
        default = "25.3.2";
      };
      languageServer = mkEnableOption "elixir-ls" // {default = true;};
      shadowAsdf = mkEnableOption "override ASDF to use Nix-provided binaries" // {default = true;};
      version = mkOption {
        type = types.nullOr types.string;
        default = "1.14.5";
      };
    };
  };
  config = {
    perSystem = {
      config,
      pkgs,
      ...
    }: let
      pkgset = inputs.beam-flakes.lib.mkPackageSet {
        inherit pkgs;
        inherit (cfg) erlangVersion;
        elixirVersion = cfg.version;
        elixirLanguageServer = cfg.languageServer;
      };
    in {
      devShells.elixir = pkgs.mkShell {
        # ockam_vault_software uses a Rust NIF
        inputsFrom = with config.devShells; [rust tooling];
        packages = with pkgset; [elixir erlang] ++ lib.optional cfg.languageServer elixir-ls;
        # support pkgconfig without duplication of effort
        inherit (config.devShells.rust) nativeBuildInputs shellHook;

        ASDF_ELIXIR_VERSION = lib.optional cfg.shadowAsdf "system";
        ASDF_ERLANG_VERSION = lib.optional cfg.shadowAsdf "system";
        inherit (config.devShells.rust) OCKAM_DISABLE_UPGRADE_CHECK RUSTFLAGS RUST_SRC_PATH;
        inherit (config.devShells.tooling) BATS_LIB;
      };
    };
  };
}
