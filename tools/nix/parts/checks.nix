{
  config = {
    perSystem = {config, ...}: {
      devShells.pre-commit = config.pre-commit.devShell;

      # https://flake.parts/options/pre-commit-hooks-nix.html
      pre-commit = {
        check.enable = true;
        settings = {
          hooks = {
            alejandra.enable = true;
            statix.enable = true;
          };

          settings = {statix.ignore = [".direnv/*"];};
        };
      };
    };
  };
}
