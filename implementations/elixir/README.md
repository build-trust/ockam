# Ockam

Ockam is a collection of protocols and toolkits for building connected
systems that you can trust.

This folder contains the Elixir implementation of Ockam.

## Build

1. Lint

```
cd implementations/elixir && make lint
```

2. Build

```
cd implementations/elixir && make build
```

3. Test

```
cd implementations/elixir && make test
```

4. Clean

```
cd implementations/elixir && make clean
```

## ockam_vault_software NIFs

`ockam_vault_software` provides a set of NIF functions Ockam depends on.
Those functions link the Rust implementation of `ockam_vault` crate.

By default this repo provides pre-build NIF files for MacOS (universal) and Linux (x86_64)

To build Ockam Elixir implementation on other architectures, Rust implementation should also be built.

Please see `ockam_vault_software/README.md` for more information.
