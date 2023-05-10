# Nix

This prototype [Nix Flake](https://zero-to-nix.com/concepts/flakes) offers a
self-contained, versioned, reproducible development environment for working on
the Elixir/Rust libraries offered within this open-source repository. This flake
is highly experimental with no stability or suitability guarantees, and not
encouraged for use by external contributors.

## Aside: Nix Use Within Ockam

A few individuals within the Ockam engineering team use Nix in some personal
capacity. Ockam is exploring use of Nix in a more intentional capacity but at
this time we have no formal plans or timeline to share regarding this subject.

## Requirements

This flake requires a [Flake-enabled installation of
Nix](https://zero-to-nix.com/start/install) and was developed using Nix 2.11 on
MacOS 13 on an Apple Silicon device. It has also been tested using NixOS 22.11
on x86_64-compatible hardware.

## Usage

Optionally, you can compliment Nix with [direnv](https://direnv.net/) and
[nix-direnv](https://github.com/nix-community/nix-direnv/) by copying the
`.envrc.sample` at the root of the repository to `/.envrc` instead, which is
gitignore'd. The `use nix` syntax within is documented in [direnv's
stdlib](https://direnv.net/man/direnv-stdlib.1.html).

If you prefer not to use `direnv`, you can instead enter the development
environment using:

```shell
# all languages included in this flake
nix develop ./tools/nix
# elixir-only
nix develop ./tools/nix#elixir
# rust-only
nix develop ./tools/nix#rust
nix develop ./tools/nix#nightly
```

No public binary cache for Nix content is offered at this time, so some
`devShell` content will build locally the first time you use these, particularly
the ones which contain an Elixir toolchain. We use Elixir/Erlang versions that
diverges from those offered by Nixpkgs, so Nix will fall back to compiling these
from source.

## Non-goals

This flake will generally not adopt any additional Nix flake inputs, libraries,
abstractions, or frameworks without significant prior discussion and approval,
and will intentionally be kept **very sparse** in this regard. Syntax that
requires knowledge beyond a general familiarity with the Nix language and
nixpkgs' library or modules system will undergo additional scrutiny.
Abstractions which use another file format or language rather than the Nix
language syntax (or JSON for static scalar data) will not accepted.
Compatibility with non-Flakes Nix usage is a non-goal and non-trivial changes to
enable this are unlikely to be accepted.

This flake is deliberately focused on providing repeatable development
environments, so there are no current intentions to provide:

- Packaged binaries/artifacts of any Ockam project
- Nix library functions for external consumption
- Nix overlays for external consumption
- Home-Manager configurations or modules
- NixOS configurations or modules
- Text editor configuration or plugins, excepting language servers' binaries

## Future Documentation Topics

### Nix Onboarding Resources
### Detailed Contents Of DevShells
### Contributors' Guide
### Flake structure

