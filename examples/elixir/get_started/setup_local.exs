Mix.install([
  {:ockam, path: "../../../implementations/elixir/ockam/ockam"},
  {:ockam_typed_cbor, override: true,
    path: "../../../implementations/elixir/ockam/ockam_typed_cbor"},
  {:ockam_vault_software,
   override: true, path: "../../../implementations/elixir/ockam/ockam_vault_software"},
  {:ranch, "~> 2.1"}
])

Application.load(:ockam)
