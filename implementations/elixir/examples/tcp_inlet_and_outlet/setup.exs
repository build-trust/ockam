Mix.install([
  {:ockam, path: "../../ockam/ockam"},
  {:ockam_typed_cbor, override: true,
    path: "../../ockam/ockam_typed_cbor"},
  {:ranch, "~> 2.1"}
])

Application.load(:ockam)
