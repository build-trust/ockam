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
        version = "2.0.0-alpha.11";
      in
        # Need to make changes?
        # https://nixos.org/manual/nixpkgs/stable/#compiling-rust-applications-with-cargo
        pkgs.rustPlatform.buildRustPackage {
          inherit pname version;

          src = pkgs.fetchFromGitHub {
            owner = "build-trust";
            repo = "tauri";
            rev = "7856354fe16197b270dfa36bc095fec33bec4cff";
            hash = "sha256-UxWT5k3ZTbydy2iW9LXuSLIfQhSafN39g566J0xWDDs=";
          };
          sourceRoot = "source/tooling/cli";

          cargoDepsName = pname;
          cargoHash = "sha256-4OKYj9rPB998JQTLi/k8ICBgYL/jcxXJT/4MAAR6wzU=";

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
