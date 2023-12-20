# Ockam

Thank you for your interest in contributing to the Ockam open source projects.

Ockam is a collection of protocols and toolkits for building connected
systems that you can trust. This folder contains the Elixir implementation of Ockam.

Please read our community's [*Code of Conduct Covenant*][conduct] and our [contributing guidelines][contributing].

To start contributing to our Elixir code, clone the Ockam repo from Github and change your current directory
to `implementations/elixir`:

```
git clone git@github.com:build-trust/ockam.git
cd implementations/elixir
```

# Get Help

Ask a question on [Discord](https://discord.ockam.io).

# Using `mix`

The standard build tool for developing Elixir projects is [`mix`](https://hexdocs.pm/mix/1.15/Mix.html).
This executable is included in the `Elixir` distribution which you can install using the instructions on [this page](https://elixir-lang.org/install.html#by-operating-system).

## Setting Elixir and Erlang versions with `asdf`

[`asdf`](https://github.com/asdf-vm/asdf) is a popular tool to manage different Elixir and Erlang versions on the same
machine. At the moment there are some inconsistencies in the versions defined in `mix.exs` files.
Some use elixir `1.10`, others use `1.13`.

You can use the following to make sure that all packages builds correctly, use the same versions of Elixir and Erlang.
```bash
asdf install elixir $ELIXIR_VERSION
asdf local elixir $ELIXIR_VERSION

asdf install erlang $ERLANG_VERSION
asdf local erlang $ERLANG_VERSION
```

(replace `$ELIXIR_VERSION` and `$ERLANG_VERSION` with the appropriate versions).

## Build

Each package in the `ockam` directory can be compiled using `mix compile`.
First you need to update the package dependencies with `mix deps.get`. For example:
```
cd ockam/ockam
mix deps.get
```
Then you can call:
```
mix compile
```

### Code formatting

The code can be formatted with `mix format`.

### Native functions

The `ockly` package provides a set of [NIF functions](https://www.erlang.org/doc/tutorial/nif.html).
Those functions are used by the `ockam` package in the implementation of secure channels.
For example:

 - `create_identity`: create a new identity, including its private key
 - `issue_credential`: create a credential, for a given identity, attested by another identity

The list of the NIF functions can be found in [this file](./ockam/ockly/lib/ockly/native.ex).

The NIF functions are implemented using Rust using a project inside the `./ockly/native/ockly` directory.
When you run `mix compile` in the `ockly` package, the underlying Rust project is then compiled using any `cargo` version
found on the `PATH`. Please refer to the [Ockam Rust project README](../rust/README.md) to install the necessary Rust tools.

## Test

The tests for a given package can be executed with (remember to `cd` into a specific package like `cd ockam/ockam`):
```
mix test
```

A specific test file can be executed with:
```
mix test test/ockam/identity_test.exs
```

It is also possible to just execute one test by specifying to function line number:
```
mix test test/ockam/identity_test.exs:19
```

This is particularly useful when a test fail because you can just copy and paste the file name and line number from
the test failure.

## Documentation

The documentation for a given package can be generated with:
```
mix docs
```

The generated documented can be opened in a browser with:
```
open _build/docs/index.html
```

## Lint

There are 4 lints available:

 - `mix lint.format`: checks that the code is formatted
 - `mix lint.credo`: runs a static code analysis executed with [`credo`](https://hexdocs.pm/credo/overview.html)
 - `mix lint`: runs both `lint.format` and `lint.credo`
 - `mix lint.dialyzer`: runs a static code analysis executed with [`credo`](https://hexdocs.pm/dialyzer/Mix.Tasks.Dialyzer.html)

# Using `make`

All the `mix` commands can be run using `make`, directly from the root directory.
Here is a list of some commands:

Command                    | Description
 ------                    | -----------
 `make elixir_clean`       | clean build files using `mix clean`
 `make elixir_deps`        | update dependencies using `mix deps.get`
 `make elixir_build`       | build the `ockam` package using `mix compile`
 `make elixir_build_ockam` | build all packages using `mix compile`
 `make elixir_test`        | run all the tests, using `mix test`
 `make elixir_test_ockam`  | run the tests for the `ockam` package, using `mix test`
 `make elixir_lint`        | run all the code lints
 `make elixir_lint_ockam`  | run the code lints on the `ockam` package

Note that these commands don't need to use the `elixir_` prefix if you first `cd` into `implementations/elixir`. Then you
can directly call: `make build`, `make test`, etc...

# Using `nix`

Our [Nix](https://nixos.org) tooling can be found in `tools/nix` from the root path directory. This has the benefit to install *all* the tools
needed to build, test and check the project. You first need to install `nix` by following the instructions [here](https://nixos.org/download#download-nix).

Then you can run any `make` command in a `nix` environment. For example here is how you can run the tests:
```
nix develop ./tools/nix#elixir --command make elixir_test
```

There is also a `make` shortcut to run any command in a `nix` environment:
```
# run the mix tests
make nix_elixir_test

# run the mix lints
make nix_elixir_lint
```

[conduct]: https://github.com/build-trust/.github/blob/main/CODE_OF_CONDUCT.md
[contributing]: https://github.com/build-trust/.github/blob/main/CONTRIBUTING.md
