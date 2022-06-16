repo = "https://github.com/build-trust/ockam.git"

Mix.install([
  {:ockam,
    git: repo, branch: "develop", force: true, sparse: "implementations/elixir/ockam/ockam"},
  {:ockam_typed_cbor, override: true,
    git: repo, branch: "develop", sparse: "implementations/elixir/ockam/ockam_typed_cbor"},
  {:ockam_vault_software, override: true,
    git: repo, branch: "develop", sparse: "implementations/elixir/ockam/ockam_vault_software"},
  {:ranch, "~> 2.1"}
])

Application.load(:ockam)
