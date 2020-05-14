defmodule Ockam.Services do
  use DynamicSupervisor

  require Logger

  def start_link([services]) when is_list(services) do
    Logger.info("Starting #{__MODULE__} with #{inspect(services)}")

    {:ok, pid} = DynamicSupervisor.start_link(__MODULE__, [], name: __MODULE__)

    for {service_name, service_config} <- services do
      case start_service(service_name, service_config) do
        {:ok, _} ->
          :ok

        {:error, reason} ->
          Logger.error(
            "Failed to start preconfigured service #{service_name}: #{inspect(reason)}"
          )

          exit(reason)
      end
    end

    {:ok, pid}
  end

  def init(_) do
    DynamicSupervisor.init(strategy: :one_for_one)
  end

  @spec start_service(atom, Keyword.t()) :: {:ok, pid} | {:error, term}
  def start_service(name, config) do
    {service, service_opts} = Keyword.pop!(config, :service)

    Logger.info("Starting service #{inspect(service)} with config: #{inspect(service_opts)}")

    meta = [name: {:via, Registry, {Ockam.Registry, to_string(name), service}}]
    spec = {service, [meta, service_opts]}
    DynamicSupervisor.start_child(__MODULE__, spec)
  end
end
