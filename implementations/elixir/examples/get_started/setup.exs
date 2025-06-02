Mix.install([
  {:ockam, path: "../../ockam/ockam"},
  {:ockam_typed_cbor, override: true,
    path: "../../ockam/ockam_typed_cbor"},
  {:ranch, "~> 2.2"}
])

Application.load(:ockam)
