## Ockam vault

This application provides NIFs to access Vault functions implemented in Rust.

The repo contains pre-build `.so` files for MacOS(universal) and Linux(x86_64), which can be found
in `priv/darwin_universal/native` and `priv/linux_x86_64/native`.

For other architectures the build process will try to re-build the NIFs and put them in `priv/native`.
Re-build requires existing of `ockam_vault` and `ockam_ffi` Rust libraries.

You can also force build those files even for MacOS and Linux by running `mix recompile.native`.
If there are some issues with the libs loading, for example.

**NOTE Custom built libs take precedence when loading. If there are lib files in `priv/native`, they will be used instead of pre-built `priv/.../native`**
