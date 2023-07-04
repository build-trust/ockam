{
  perSystem = {
    config,
    lib,
    pkgs,
    ...
  }: {
    devShells.tauri = pkgs.mkShell {
      inputsFrom = [config.devShells.rust];
      packages = [config.packages.tauri-cli];
    };

    packages = {
      tauri-cli = let
        pname = "tauri-cli";
        # NOTE: Bumping version must always be accompanied by updating the two hashes below
        # https://github.com/tauri-apps/tauri/releases
        version = "2.0.0-alpha.10";
      in
        # Need to make changes?
        # https://nixos.org/manual/nixpkgs/stable/#compiling-rust-applications-with-cargo
        pkgs.rustPlatform.buildRustPackage {
          inherit pname version;

          src = pkgs.fetchFromGitHub {
            owner = "tauri-apps";
            repo = "tauri";
            rev = "tauri-v${version}";
            hash = "sha256-WOxl+hgzKmKXQryD5tH7SJ9YvZb9QA4R+YUYnarnhKA=";
          };
          sourceRoot = "source/tooling/cli";

          cargoDepsName = pname;
          cargoHash = "sha256-MQmEOdQWyRbO+hogGQA2xjB9mezq21FClvscs1oWYLE=";

          buildInputs =
            [pkgs.openssl]
            ++ lib.optionals pkgs.stdenv.isDarwin (with pkgs.darwin.apple_sdk.frameworks; [
              CoreServices
            ]);
          nativeBuildInputs = [pkgs.pkg-config];

          # Skip upstream test suite
          doCheck = false;
        };
    };
  };
}
