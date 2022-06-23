## Ockam vault

This application provides NIFs to access Vault functions implemented in Rust.

## NIF libs

This application requires a `libockam_elixir_ffi.so` NIF to function.

Ockam release provides pre-built NIF libraries for MacOS(universal) and Linux(x86_64_gnu) in https://github.com/build-trust/ockam/releases/latest

If you run this application on supported architectures, it will download the libraries from release.
For other architectures the build process will try to re-build the NIFs and put them in `priv/native`.

**HEX packages are shipped with release NIFs of same version number as the HEX package.**

## Rebuilding NIFs

NIFs are built using CMake

Build requires existing and built of `ockam_vault` and `ockam_ffi` Rust libraries, you can build them in `implementations/rust/ockam/ockam_ffi` by running `cargo build --release`.

You can force build the NIFs even for MacOS and Linux by running `mix recompile.native`.
If there are some issues with the libs loading, for example.

**NOTE Custom built libs take precedence when loading. If there are lib files in `priv/native`, they will be used instead those downloaded to `priv/.../native`**


## Publishing the package

To publish the current version:

`mix hex.publish`

Publish will download release libs from the same version as the package.

To build a new version (without changing mix.exs):

`VERSION=<ockam_version> mix hex.publish`


