# Develop

Thank you for your interest in contributing to the Ockam open source projects.

Please read our community's [*Code of Conduct Covenant*][conduct] and
our [contributing guidelines][contributing].

To start contributing to our rust code, clone the Ockam repo from Github and
change your current directory to `ockam/implementations/rust`:

```
git clone git@github.com:build-trust/ockam.git
cd ockam/implementations/rust
```

## Setup

If you don't already have it, you will need Rust stable and nightly toolchains
installed. To get them install [rustup](https://rustup.rs) and then use it
setup the `stable` and `nightly` rust toolchains:

```
rustup toolchain install stable
rustup toolchain install nightly
```

Refer Rust [documentation][rustup-manage-versions] on managing and
updating rust versions.

## Test

Once you make some changes in a crate and write some tests, you can run them
with:

```
cargo test
```

Many Ockam crates have a Cargo feature named `"std"` that is enabled by default.
In order to test such a crate in a `no_std` context run:

```
cargo test --no-default-features
```

## Lint

To validate that the new code you've added is formatting according to
our project conventions:

```
cargo fmt --all -- --check
```

You can ask cargo to automatically fix any formatting inconsistencies
by running:

```
cargo fmt
```

To run clippy to catch any common mistakes:

Add it to the nightly toolchain via rustup and then run it with `cargo +nightly`

```
rustup component add clippy --toolchain nightly
cargo +nightly clippy --all-targets --all-features -- -D warnings
```

## Documentation

Generate rust documentation:

```
cargo doc
```

## Code Coverage

Get a code coverage report:

```
cargo +nightly install grcov

env CARGO_INCREMENTAL=0 RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort --cfg tokio_unstable" RUSTDOCFLAGS="-Cpanic=abort" cargo +nightly test

grcov --llvm . -s . --binary-path ./target/debug/ -t html --branch --ignore-not-existing -o ./target/debug/coverage/

open target/debug/coverage/index.html
```

## Crate Dependency Graph

Generate a crate dependency graph:

```
cargo install cargo-deps
cargo deps --all-deps | dot -Tpng > graph.png
```

## Module Dependency Graph

Generate a module dependency graph:

```
rustup run nightly cargo install cargo-modules
cargo +nightly modules --orphans graph | dot -Tpng > modules.png
```

## Dependency Licenses

See licenses used by all dependencies:

```
cargo install cargo-license
cargo license
```

See a unique list of all dependencies, this is useful in confirming that
we are only adding dependencies that a permissive license like an
Apache, MIT or BSD variant.

```
cargo license --json | jq ".[] | .license" | sort | uniq
```

## Get Help

Ask a question on [Github Discussions](https://github.com/build-trust/ockam/discussions)



[conduct]: https://www.ockam.io/learn/how-to-guides/high-performance-team/conduct
[contributing]: https://www.ockam.io/learn/how-to-guides/contributing/CONTRIBUTING
[rustup-manage-versions]: https://doc.rust-lang.org/nightly/edition-guide/rust-2018/rustup-for-managing-rust-versions.html#rustup-for-managing-rust-versions
