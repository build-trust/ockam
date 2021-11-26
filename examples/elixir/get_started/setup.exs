repo = "https://github.com/ockam-network/ockam.git"

Mix.install([
  {:ockam,
    git: repo, branch: "hairyhum/refactor-message-type", sparse: "implementations/elixir/ockam/ockam"},
  {:ockam_vault_software, override: true,
    git: repo, branch: "develop", sparse: "implementations/elixir/ockam/ockam_vault_software"},
  {:ranch, "~> 2.1"}
])

Application.load(:ockam)
