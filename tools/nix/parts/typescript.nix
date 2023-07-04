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
        # TODO: Latest is 8.6.6 as of 2023-07-02
        pnpm = let
          pname = "pnpm";
          # NOTE: Bumping version must always be accompanied by changing the sha512 hash
          version = "6.7.6";
          # https://raw.githubusercontent.com/nixos/nixpkgs/nixpkgs-unstable/pkgs/development/node-packages/node-packages.nix
          src = pkgs.fetchurl {
            url = "https://registry.npmjs.org/pnpm/-/${pname}-${version}.tgz";
            sha512 = "sha512-VhO6zVIuhVkKXP3kWMZs9W5b3rhcztq524WoAc9OEwjmj7SiKyp0UNltaaLR0VRjFGJPuQOcqDbNkWwzao6dUw==";
          };
        in
          pkgs.nodePackages.pnpm.override {
            inherit src version;
            node = config.packages.nodejs;
          };
      };
    };
  };
}
