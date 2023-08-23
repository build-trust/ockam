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

      rustAnalyzer = mkEnableOption "install nightly rust-analyzer via Nix" // {default = true;};
      suggestedCargoPlugins = mkEnableOption "extra cargo plugins";
      version = mkOption {
        type = types.nullOr (types.strMatching "^([0-9]+)\.([0-9]+)(\.([0-9]+))$");
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
        compilerTools = with pkgs; [
          clang
          cmake
          lld
        ];

        nightlyToolchain = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
          targets = [ "thumbv7em-none-eabihf" ];
        });
        toolchain = pkgs.rust-bin.fromRustupToolchainFile ../../../rust-toolchain.toml;

        nativeLibs = with pkgs;
          [(lib.getDev openssl)]
          ++ lib.optionals stdenv.isLinux [
            dbus
            webkitgtk_4_1
            (lib.getDev systemd)
          ]
          ++ lib.optionals stdenv.isDarwin (with darwin.apple_sdk.frameworks; [
            AppKit
            CoreBluetooth
            IOKit
            pkgs.libiconv
            Security
            WebKit
          ]);

        cargoPlugins = with pkgs;
          [
            # used in DEVELOP.md
            cargo-deny
            cargo-deps
            cargo-license
            cargo-modules
            cargo-nextest
            cargo-readme
            dprint
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
          CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER = lib.getExe pkgs.clang;
          OCKAM_DISABLE_UPGRADE_CHECK = lib.optional cfg.disableUpgradeCheck true;
          RUSTFLAGS = "--cfg tokio_unstable -Cdebuginfo=0 -Dwarnings";
          CARGO_INCREMENTAL = 0;
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

          inherit (envVars) CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER OCKAM_DISABLE_UPGRADE_CHECK RUSTFLAGS;

          DYLD_FALLBACK_LIBRARY_PATH = "${toolchain}/lib";
          RUST_SRC_PATH = lib.optional cfg.rustAnalyzer "${toolchain}/lib/rustlib/src/rust/library";
        };

        rust_nightly = pkgs.mkShell {
          inputsFrom = [config.devShells.tooling];
          nativeBuildInputs = [pkgs.pkgconfig];
          packages =
            [
              nightlyToolchain
            ]
            ++ nightlyTooling
            ++ sharedInputs;
          inherit (envVars) CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER OCKAM_DISABLE_UPGRADE_CHECK RUSTFLAGS;
          DYLD_FALLBACK_LIBRARY_PATH = "${nightlyToolchain}/lib";
          RUST_SRC_PATH = lib.optional cfg.rustAnalyzer "${nightlyToolchain}/lib/rustlib/src/rust/library";
        };
      };
    };
  };
}
