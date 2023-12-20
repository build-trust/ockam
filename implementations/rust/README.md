# Ockam

Thank you for your interest in contributing to the Ockam open source projects.

Ockam is a collection of protocols and toolkits for building connected
systems that you can trust. This folder contains the Rust implementation of Ockam.

Please read our community's [*Code of Conduct Covenant*][conduct] and our [contributing guidelines][contributing].

To start contributing to our rust code, clone the Ockam repo from Github and change your current directory
to `implementations/rust`:

```
git clone git@github.com:build-trust/ockam.git
cd implementations/rust
```

# Get Help

Ask a question on [Discord](https://discord.ockam.io).

# Using `cargo`

## Setup

If you don't already have it, you will need Rust stable and nightly toolchains installed.
To get them install [rustup](https://rustup.rs) and then use it setup the `stable` and `nightly` rust toolchains:

```
rustup toolchain install stable
rustup toolchain install nightly
```

Refer Rust [documentation][rustup-manage-versions] on managing and updating rust versions.

## Build

The code can be built with:

```
cargo build
```

This will produce an `ockam` executable at `target/debug/ockam`.

You can also typecheck the code, as you modify it, with:
```
cargo watch -x check
```

or, if you want to include the test code:
```
cargo watch -x "check --tests"
```

### `no_std`

The `ockam` crate has a Cargo feature named `"std"` that is enabled by default.
However it can also be built in a `no_std` context with:

```
cargo build --target thumbv7em-none-eabihf --package ockam --no-default-features --features 'no_std alloc software_vault'
```

## Test

Once you make some changes in a crate and write some tests, you can run them with:

```
cargo test
```

### Concurrent tests

There is also a way to run all the tests concurrently, to run them faster, with `nextest`:
```
cargo --config-file tools/nextest/.config/nextest.toml run --no-fail-fast
```

`--no-fail-fast` runs all the tests, regardless of failures.

### BATS tests

The Ockam system tests are implemented using a command-line test tool: [bats](https://bats-core.readthedocs.io).
Please consult [the `bats` installation instructions to install it on your system](https://bats-core.readthedocs.io/en/stable/installation.html).

Once `bats` is installed you can run the `bats` test suite with:
```
# --jobs 4 run the tests concurrently using 4 threads
bats implementations/rust/ockam/ockam_command/tests/bats --jobs 4
```

The `implementations/rust/ockam/ockam_command/tests/bats` directory contains several tests files, covering different
functionalities. You can specify which file you want to test and even add a filter for the subset of tests you want to run:

```
# run only the CRUD tests in the vault test suite
bats implementations/rust/ockam/ockam_command/tests/bats/vault.bats --jobs 4 --filter "CRUD"
```

## Lint

To validate that the new code you've added is formatted according to our project conventions:

```
cargo fmt --check
```

You can ask cargo to automatically fix any formatting inconsistencies by running:

```
cargo fmt
```

`clippy` is a Cargo plugin that can catch many common mistakes. You can run on the Ockam code with:
```
cargo clippy --no-deps --all-targets -- -D warnings
```

## Documentation

Generate rust documentation:

```
cargo doc
```

### README files

The `README.md` files for a given crate is generated from the documentation header in the top-level `lib.rs` file,
using the `cargo readme` plugin.

That plugin must be installed with `cargo install cargo-readme`. Then a `README` file in a given crate can be updated with:
```
# update the README file for the ockam_identity crate
cd implementations/rust/ockam_identity
cargo readme --project-root ockam_identity --template ../README.tpl -o README.md
```

## Code Coverage

Get a code coverage report:

```
# install the grcov binary
cargo install grcov

# run the cargo tests with a coverage profile
# this needs to run with +nightly
export CARGO_INCREMENTAL=0                                                                                                                                                                                                                                                                                                                                                                            ockam dotfiles flox/default default
export RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests"
cargo +nighlty test --profile coverage

# generate the report
~/.cargo/bin/grcov --llvm . -s . --binary-path ./target/debug/ -t html --branch --ignore-not-existing -o ./target/debug/coverage/

# open the report
open target/debug/coverage/index.html
```

It seems that some directories are not included in coverage data when running `cargo test`.
In order to still get a coverage report for those directories you can use `cargo nextest` instead:

```
# install the llvm-cov cargo plugin
cargo install cargo-llvm-cov

# run the tests and produce an HTML report
cargo llvm-cov nextest -p ockam_api --html

# open the coverage report
open target/llvm-cov/html/index.html
```

## Crate Dependency Graph

Generate a crate dependency graph of Ockam's crates:

```
cargo install cargo-depgraph
cargo depgraph --workspace-only | dot -Tpng > graph.png
```

## Module Dependency Graph

Generate a module dependency graph:

```
cargo install cargo-modules
cargo modules --orphans graph | dot -Tpng > modules.png
```

## Dependency Licenses

See licenses used by all dependencies:

```
cargo install cargo-license
cargo license
```

See a unique list of all dependencies, this is useful in confirming that we are only adding dependencies that have
permissive license like an Apache, MIT or BSD variant.

```
cargo license --json | jq ".[] | .license" | sort | uniq
```

# Using `make`

Many `cargo` commands have an equivalent support using `make`. Here are a few examples, you can find more in [implementations/rust/Makefile](./Makefile):

 Command                       | Description
 ------                        | -----------
 `make rust_clean`             | clean build files
 `make rust_build`             | build all crates
 `make rust_test`              | run the tests, using `cargo test`
 `make rust_nexttest`          | run the tests, using `cargo nextest`
 `make rust_bats`              | run the `bats` test suite
 `make rust_bats_vault`        | run the `bats` `vault` test suite
 `make rust_update_readmes`    | update the `README` files in all crates, based on the documentation header in `<crate>/src/lib.rs`
 `make rust_cargo_fmt`         | format the code
 `make rust_lint`              | run all the code lints
 `make rust_lint_clippy`       | run the clippy lints
 `make rust_lint_cargo_readme` | check that the README files are up to date
 `make rust_check_no_std`      | check that the `ockam` crate can be compiled with the `no_std` feature

Note that these commands don't need to use the `rust_` prefix if you first `cd` into `implementations/rust`. Then you
can directly call: `make build`, `make test`, etc...

# Using `nix`

Our [Nix](https://nixos.org) tooling can be found in `tools/nix` from the root path directory. This has the benefit to install *all* the tools
needed to build, test and check the project. You first need to install `nix` by following the instructions [here](https://nixos.org/download#download-nix).

Then you can run any `make` command in a `nix` environment. For example here is how you can run the tests:
```
nix develop ./tools/nix#rust --command make rust_test
```

There is also a `make` shortcut to run any command in a `nix` environment:
```
# run the cargo tests
make nix_rust_test

# run the bats tests
make nix_rust_bats
```

# Using an IDE

We recommend using [RustRover](https://www.jetbrains.com/rust) or VSCode with [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
to benefit from code navigation, refactoring and automated formatting.

[conduct]: https://github.com/build-trust/.github/blob/main/CODE_OF_CONDUCT.md
[contributing]: https://github.com/build-trust/.github/blob/main/CONTRIBUTING.md
[rustup-manage-versions]: https://doc.rust-lang.org/nightly/edition-guide/rust-2018/rustup-for-managing-rust-versions.html#rustup-for-managing-rust-versions
