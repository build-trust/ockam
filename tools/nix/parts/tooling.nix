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
      ];

      BATS_LIB = "${config.packages.bats}/share/bats";

      shellHook = ''
        export LANG=en_US.UTF-8
        export HOME=$PWD/.home
        export SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt
      '';
    };

    # Responsible only for formatting this flake itself
    formatter = pkgs.alejandra;

    packages.bats = pkgs.bats.withLibraries (p: [p.bats-assert p.bats-file p.bats-support]);
  };
}
