{
  config = {
    perSystem = {
      config,
      lib,
      pkgs,
      ...
    }: {
      devShells.pre-commit = config.pre-commit.devShell;

      # https://flake.parts/options/pre-commit-hooks-nix.html
      pre-commit = {
        check.enable = true;
        settings = {
          hooks = {
            alejandra.enable = true;

            # not implemented upstream in pre-commit-hooks.nix
            commitlint = {
              enable = true;
              entry = "${lib.getExe pkgs.commitlint} --config ${../../commitlint/commitlint.config.js} --edit";
              name = "commitlint";
              stages = ["commit-msg"];
              types = [];
            };

            mix-format = {
              enable = true;
              entry = lib.mkForce "${config.packages.elixir}/bin/mix format";
              excludes = ["^examples/elixir"];
            };

            rustfmt.enable = true;

            statix.enable = true;
          };

          settings = {statix.ignore = [".direnv/*"];};
        };
      };
    };
  };
}
