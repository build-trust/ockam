defmodule Ockam.App do
  use Application

  require Logger

  alias Ockam.Vault

  def start(_type, _args) do
    Logger.info("Starting #{__MODULE__}")
    Logger.info("Initializing Ockam Vault..")

    vault_config = Application.get_env(:ockam, :vault, [])
    Vault.init_vault!(vault_config)

    transports = Application.get_env(:ockam, :transports, [])

    children = [
      {Ockam.Transport.Supervisor, [transports]}
    ]

    Supervisor.start_link(children, strategy: :one_for_one, name: __MODULE__)
  end
end
