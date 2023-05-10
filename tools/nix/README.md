# Nix

This [Nix Flake](https://zero-to-nix.com/concepts/flakes) offers a
self-contained, versioned, reproducible development environment for Ockam.

## Setup

* Setup a [Flake-enabled installation of Nix](https://zero-to-nix.com/start/install).
* Optionally setup [direnv](https://direnv.net/) and [nix-direnv](https://github.com/nix-community/nix-direnv/).

If you prefer not to use `direnv`, you can instead enter the development environment using:

```shell
# all languages included in this flake
nix develop ./tools/nix
```

```shell
# elixir-only
nix develop ./tools/nix#elixir
```

```shell
# rust-only
nix develop ./tools/nix#rust
nix develop ./tools/nix#rust_nightly
```
