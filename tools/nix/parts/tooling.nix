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
        config.packages.tomlq
        commitlint
        curl
        git
        graphviz
        jq
        which
        git-cliff
        cargo-release
        cargo-readme
        # For cargo search query
        cargo
        cmake
        gnupg
      ];

      BATS_LIB = "${config.packages.bats}/share/bats";
    };

    packages = {
      tomlq = let
        pname = "tomlq";
        version = "0.1.0";
      in
        pkgs.rustPlatform.buildRustPackage {
          inherit pname version;
          src = pkgs.fetchFromGitHub {
            owner = "jamesmunns";
            repo = "tomlq";
            rev = "66b1ee60d559dd2881fc8a4e92918fb7a65bb561";
            hash = "sha256-xrdpcVywhxueNnu1vTr5o/79VyHA6BEBIAXh7Y9J/vo=";
          };

          cargoDepsName = pname;
          cargoHash = "sha256-F11PsY11KOFjCnd52PjlQ3DzTbl38rzKaJfi6Y4eurM=";
        };
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
