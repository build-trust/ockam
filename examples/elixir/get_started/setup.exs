Mix.install([
  {:ockam, path: "../../../implementations/elixir/ockam/ockam"},
  {:ockam_typed_cbor, override: true,
    path: "../../../implementations/elixir/ockam/ockam_typed_cbor"},
  {:ranch, "~> 2.1"}
])

Application.load(:ockam)
