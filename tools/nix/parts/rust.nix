{
  config,
  inputs,
  lib,
  ...
}: let
  inherit (lib) mkEnableOption mkOption types;
  cfg = config.ockam.rust;
in {
  options = {
    ockam.rust = {
      disableUpgradeCheck = mkEnableOption "disable upgrade checks by CLI" // {default = true;};
      extraCargoPlugins = mkOption {
        type = types.listOf types.package;
        default = [];
      };
      moldLinkerLinux = mkEnableOption "mold linker for Linux hosts" // {default = true;};
      # upstream still considers this experimental
      moldLinkerDarwin = mkEnableOption "mold linker for MacOS hosts";

      rustAnalyzer = mkEnableOption "install nightly rust-analyzer via Nix" // {default = true;};
      suggestedCargoPlugins = mkEnableOption "extra cargo plugins";
      version = mkOption {
        type = types.nullOr (types.strMatching "^([0-9]+)\.([0-9]+)(\.([0-9]+))$");
        default = "1.69.0";
      };
    };
  };

  config = {
    perSystem = {
      config,
      lib,
      pkgs,
      system,
      ...
    }: {
      devShells = let
        moldEnabled = (pkgs.stdenv.isLinux && cfg.moldLinkerLinux) || (pkgs.stdenv.isDarwin && cfg.moldLinkerDarwin);

        compilerTools = with pkgs;
          [
            clang
            cmake
            lld
          ]
          ++ lib.optional moldEnabled mold;

        # NOTE: this is the latest *in the data from rust-overlay input*
        nightlyToolchain = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default);
        toolchain = pkgs.rust-bin.stable.${cfg.version}.default;

        nativeLibs = with pkgs;
          [(lib.getDev openssl)]
          ++ lib.optionals stdenv.isLinux [
            dbus
            (lib.getDev systemd)
          ]
          ++ lib.optionals stdenv.isDarwin (with darwin.apple_sdk.frameworks; [
            AppKit
            CoreBluetooth
            IOKit
            pkgs.libiconv
            Security
          ]);

        cargoPlugins = with pkgs;
          [
            # used in DEVELOP.md
            cargo-deny
            cargo-deps
            cargo-license
            cargo-modules
          ]
          ++ lib.optionals cfg.suggestedCargoPlugins [
            bacon
            cargo-cache
            cargo-outdated
            cargo-sweep
            cargo-watch
            watchexec
          ]
          ++ cfg.extraCargoPlugins;

        devTools = cargoPlugins ++ lib.optional cfg.rustAnalyzer pkgs.rust-analyzer;

        nightlyTooling = with pkgs; [
          grcov
        ];

        sharedInputs = compilerTools ++ nativeLibs ++ devTools;

        envVars = {
          OCKAM_DISABLE_UPGRADE_CHECK = lib.optional cfg.disableUpgradeCheck true;
          RUSTFLAGS =
            lib.optional moldEnabled "-C link-arg=-fuse-ld=${pkgs.mold}/bin/mold";
        };
      in {
        rust = pkgs.mkShell {
          inputsFrom = [config.devShells.tooling];
          nativeBuildInputs = [pkgs.pkgconfig];
          packages =
            [
              toolchain
            ]
            ++ sharedInputs;

          inherit (config.devShells.tooling) BATS_LIB;

          inherit (envVars) OCKAM_DISABLE_UPGRADE_CHECK RUSTFLAGS;

          RUST_SRC_PATH = lib.optional cfg.rustAnalyzer "${toolchain}/lib/rustlib/src/rust/library";
        };

        nightly = pkgs.mkShell {
          nativeBuildInputs = [pkgs.pkgconfig];
          packages =
            [
              nightlyToolchain
            ]
            ++ nightlyTooling
            ++ sharedInputs;
          inherit (envVars) OCKAM_DISABLE_UPGRADE_CHECK RUSTFLAGS;
          RUST_SRC_PATH = lib.optional cfg.rustAnalyzer "${nightlyToolchain}/lib/rustlib/src/rust/library";
        };
      };
    };
  };
}
