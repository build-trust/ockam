defmodule Ockam.App do
  use Application

  require Logger

  def start(_type, _args) do
    Logger.info("Starting #{__MODULE__}")

    transports = Application.get_env(:ockam, :transports, [])

    children = [
      {Ockam.Transport.Supervisor, [transports]}
    ]

    Supervisor.start_link(children, strategy: :one_for_one, name: __MODULE__)
  end
end
