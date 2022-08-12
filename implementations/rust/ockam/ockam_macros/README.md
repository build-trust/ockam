# ockam_macros

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

This crate provides shared macros.

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam_macros = "0.17.0"
```

All macros except for those used exclusively for testing purposes are re-exported by the `ockam` crate, so you may see examples where the macros are exported from `ockam_macros` if are related to tests or from `ockam` in any other case.

You can read more about how to use the macros and the supported attributes by each of them in the [crate documentation](https://docs.rs/ockam_macros).

### AsyncTryClone

Implements the `AsyncTryClone` trait as defined in [`ockam_core::AsyncTryClone`](https://docs.rs/ockam_core/latest/ockam_core/traits/trait.AsyncTryClone.html).

```rust
#[derive(ockam::AsyncTryClone)]
pub struct MyStruct {
    a: u32,
}
```

### Message

Implements the `Message` trait as defined in [`ockam_core::Message`](https://docs.rs/ockam_core/latest/ockam_core/trait.Message.html).

```rust
#[derive(ockam::Message, Deserialize, Serialize)]
pub struct MyStruct {
    a: u32,
}
```

### Node

Transforms the `main` function into an async function that sets up a node and provides a `Context` to interact with it.

```rust
#[ockam::node]
async fn main(mut ctx: ockam::Context) -> ockam::Result<()> {
    // ...
}
```

If you are executing your code in a `no_std` platform that doesn't support a `main` entry point, you must use the `no_main` attribute:

```rust
#[ockam::node(no_main)]
async fn main(mut ctx: ockam::Context) -> ockam::Result<()> {
    // ...
}
```

### Tests

To write node-related tests:

```rust
#[ockam::test]
async fn main(mut ctx: ockam::Context) -> ockam::Result<()> {
    // ...
}
```

To write vault-related tests:

```rust
use ockam_vault::Vault;

fn new_vault() -> Vault {
    Vault::default()
}

#[ockam_macros::vault_test]
fn hkdf() {}
```

## Develop

Due to dependencies constraints, the tests for all the macros contained in this crate are located in the `ockam` crate.

To test changes done in any of the macros you can use the `macro_expand_playground` from the `ockam` crate to see how a macro expands
a given input.

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[crate-image]: https://img.shields.io/crates/v/ockam_macros.svg
[crate-link]: https://crates.io/crates/ockam_macros

[docs-image]: https://docs.rs/ockam_macros/badge.svg
[docs-link]: https://docs.rs/ockam_macros

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/build-trust/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/build-trust/ockam/discussions

