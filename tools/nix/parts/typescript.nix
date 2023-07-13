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
        default = "18_x";
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
          if pkgs ? "nodejs-${cfg.nodeVersion}"
          then pkgs."nodejs-${cfg.nodeVersion}"
          else throw "unsupported nodejs version for nixpkgs: ${cfg.nodeVersion}";
        pnpm = pkgs.nodePackages.pnpm.override {
          node = config.packages.nodejs;
        };
      };
    };
  };
}
