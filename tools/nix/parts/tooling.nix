_: {
  perSystem = {
    config,
    lib,
    pkgs,
    system,
    ...
  }: {
    devShells.tooling = pkgs.mkShell {
      packages = with pkgs; [
        config.packages.bats
        commitlint
        curl
        git
        graphviz
        jq
        which
      ];

      BATS_LIB = "${config.packages.bats}/share/bats";
    };

    # Responsible only for formatting this flake itself
    formatter = pkgs.alejandra;

    packages.bats = pkgs.bats.withLibraries (p: [p.bats-assert p.bats-file p.bats-support]);

    packages.shfmt-all = pkgs.writeShellApplication {
      name = "shfmt-all";
      runtimeInputs = with pkgs; [findutils gitMinimal shfmt];
      text = ''
        git ls-files ':!:./demos/**' '*\.sh' '*\.bash' '*\.bats' | xargs shfmt --diff
      '';
    };
  };
}
