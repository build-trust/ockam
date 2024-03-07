# OckamRustElixirNifs

**TODO: Add description**

## Installation

If [available in Hex](https://hex.pm/docs/publish), the package can be installed
by adding `ockam_rust_elixir_nifs` to your list of dependencies in `mix.exs`:

```elixir
def deps do
  [
    {:ockam_rust_elixir_nifs, "~> 0.117.0"}
  ]
end
```

Documentation can be generated with [ExDoc](https://github.com/elixir-lang/ex_doc)
and published on [HexDocs](https://hexdocs.pm). Once published, the docs can
be found at <https://hexdocs.pm/ockam_rust_elixir_nifs>.

## Using NIF

NIFs are built during every Ockam release and used in production, e.g. during [healthcheck docker build](https://github.com/build-trust/ockam/blob/develop/tools/docker/healthcheck/Dockerfile), to use a precompiled NIF, we need to set `OCKAM_DOWNLOAD_NIF` to `true or 1` which will download the NIF from our GitHub release.

## Updating Precompile NIF Version

When using a precompiled NIF, we compare the SHA of the precompiled NIF downloaded from GitHub release with that stored in the `checksum-Elixir.ockam_rust_elixir_nifs.Native.exs` file. To update the default precompiled NIF version to use in the ockam_rust_elixir_nifs library, we need to update the SHASum of supported NIF architechtures in the `checksum-Elixir.ockam_rust_elixir_nifs.Native.exs` file.
