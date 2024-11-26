_: {
  perSystem = {
    config,
    lib,
    pkgs,
    system,
    ...
  }: {
    packages.uploadserver = pkgs.python311Packages.buildPythonPackage rec {
      pname = "uploadserver";
      version = "5.2.0";
      src = pkgs.python311Packages.fetchPypi {
        inherit pname version;
        sha256 = "sha256-M2gHbyj7HAbe0nuQeV5fwuejend5/Ksb15oaUzQXUfU=";
      };
      doCheck = false; # Disable tests
    };

    devShells.tooling = pkgs.mkShell {
      packages = with pkgs; [
        broot
        config.packages.bats
        config.packages.uploadserver
        commitlint
        curl
        git
        graphviz
        jq
        parallel
        which
        socat
      ] ++ lib.optionals stdenv.isLinux [
        nettools
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
        git ls-files ':!:./examples/command/**' '*\.sh' '*\.bash' '*\.bats' | xargs shfmt --diff
      '';
    };
  };
}
