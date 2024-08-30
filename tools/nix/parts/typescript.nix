{
  config,
  lib,
  ...
}: let
  inherit (lib) mkOption types;
  cfg = config.ockam.typescript;
in {
  options = {
    ockam.typescript = {
      nodeVersion = mkOption {
        type = types.str;
        default = "18";
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
      devShells = {
        typescript = pkgs.mkShell {
          buildInputs = with config.packages; [nodejs pnpm];
        };
      };

      packages = {
        nodejs =
          if pkgs ? "nodejs_${cfg.nodeVersion}"
          then pkgs."nodejs_${cfg.nodeVersion}"
          else throw "unsupported nodejs version for nixpkgs: ${cfg.nodeVersion}";
        pnpm = pkgs.pnpm;
      };
    };
  };
}
