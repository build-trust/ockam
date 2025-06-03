_: {
  perSystem = {
    config,
    lib,
    pkgs,
    ...
  }: {
    devShells.python = pkgs.mkShell {
      packages = with pkgs; [
        uv
        ollama
        (python312.withPackages(ps: with ps; [ pytest docstring-parser psycopg aiohttp dill ]))
      ];

      # Add rust and shell tool to the Python dev shell
      inputsFrom = with config.devShells; [rust tooling];
      # support pkgconfig without duplication of effort
      inherit (config.devShells.rust) nativeBuildInputs;

      inherit (config.devShells.rust) RUSTFLAGS RUST_SRC_PATH LIBCLANG_PATH;
      inherit (config.devShells.tooling) BATS_LIB;

      shellHook = ''
        ${config.devShells.rust.shellHook}
      '';
    };
  };
}
