defmodule Ockam.Transport.Supervisor do
  use DynamicSupervisor

  require Logger

  def start_link([transports]) when is_list(transports) do
    Logger.info("Starting #{__MODULE__} with #{inspect(transports)}")

    {:ok, pid} = DynamicSupervisor.start_link(__MODULE__, [], name: __MODULE__)

    for {transport_name, transport_config} <- transports do
      case start_transport(transport_name, transport_config) do
        {:ok, _} ->
          :ok

        {:error, reason} ->
          Logger.error(
            "Failed to start preconfigured transport #{transport_name}: #{inspect(reason)}"
          )

          exit(reason)
      end
    end

    {:ok, pid}
  end

  def init(_) do
    DynamicSupervisor.init(strategy: :one_for_one)
  end

  @spec start_transport(atom, Keyword.t()) :: {:ok, pid} | {:error, term}
  def start_transport(name, config) do
    {transport, transport_opts} = Keyword.pop!(config, :transport)

    Logger.info(
      "Starting transport #{inspect(transport)} with config: #{inspect(transport_opts)}"
    )

    meta = [name: name]
    spec = {transport, [meta, transport_opts]}
    DynamicSupervisor.start_child(__MODULE__, spec)
  end
end
