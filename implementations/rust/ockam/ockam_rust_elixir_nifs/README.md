# ockam_rust_elixir_nifs

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

This library builds a NIF for our Elixir library
To build the NIF module:

- Your NIF will now build along with your project.

### To load the NIF:

```elixir
defmodule Ockly do
  use Rustler, otp_app: :ockly, crate: "ockly"

  # When your NIF is loaded, it will override this function.
  def add(_a, _b), do: :erlang.nif_error(:nif_not_loaded)
end
```

### Examples

[This](https://github.com/rusterlium/NifIo) is a complete example of a NIF written in Rust.

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam_rust_elixir_nifs = "0.1.0"
```

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam

[crate-image]: https://img.shields.io/crates/v/ockam_rust_elixir_nifs.svg
[crate-link]: https://crates.io/crates/ockam_rust_elixir_nifs

[docs-image]: https://docs.rs/ockam_rust_elixir_nifs/badge.svg
[docs-link]: https://docs.rs/ockam_rust_elixir_nifs

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/build-trust/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/build-trust/ockam/discussions
