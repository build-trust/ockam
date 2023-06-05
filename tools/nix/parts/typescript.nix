{
  config,
  inputs,
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
      pnpmVersion = mkOption {
        type = types.str;
        default = "6.7.6";
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
          buildInputs = [
            pkgs.nodejs
            (pkgs.nodePackages.pnpm.override { version = cfg.pnpmVersion; })
          ];
        };
      };
    };
  };
}
