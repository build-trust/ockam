with import <nixpkgs> {};

stdenv.mkDerivation {
  name = "ockam";
  buildInputs = with pkgs; [
    cargo clangStdenv rust-analyzer rustc rustfmt cargo-expand

    openjdk gradle cmake elixir
  ];
}
