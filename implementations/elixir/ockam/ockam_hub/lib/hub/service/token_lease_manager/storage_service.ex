defmodule Ockam.TokenLeaseManager.StorageService do
  @moduledoc false
  @type lease :: Ockam.TokenLeaseManager.Lease.t()
  @type reason :: any()
  @type lease_id :: String.t()
  @type options :: any()
  @type storage_conf :: map()

  @callback handle_init(options) :: {:ok, pid()} | {:error, reason}
  @callback handle_save(storage_conf, lease) :: :ok | {:error, reason}
  @callback handle_get(storage_conf, lease_id) :: {:ok, lease} | {:error, reason}
  @callback handle_remove(storage_conf, lease_id) :: :ok | {:error, reason}
  @callback handle_get_all(storage_conf) :: {:ok, [lease]} | {:error, reason}

  defmacro __using__(_opts) do
    quote do
      @behaviour Ockam.TokenLeaseManager.StorageService
      use GenServer

      require Logger

      @name :storage_service

      def start_link({token_cloud_service, token_cloud_service_address}) do
        options =
          Application.get_env(:ockam_hub, :token_manager, [])
          |> Keyword.get(:storage_service_options, [])

        GenServer.start_link(
          __MODULE__,
          {token_cloud_service, token_cloud_service_address, options},
          name: @name
        )
      end

      def save(lease), do: GenServer.call(@name, {:save, lease})
      def get(lease_id), do: GenServer.call(@name, {:get, lease_id})
      def remove(lease_id), do: GenServer.call(@name, {:remove, lease_id})
      def get_all(), do: GenServer.call(@name, :get_all)

      @impl true
      def init(opts), do: handle_init(opts)

      @impl true
      def handle_call({:save, lease}, _from, storage_conf),
        do: {:reply, handle_save(storage_conf, lease), storage_conf}

      def handle_call({:get, lease_id}, _from, storage_conf),
        do: {:reply, handle_get(storage_conf, lease_id), storage_conf}

      def handle_call({:remove, lease_id}, _from, storage_conf),
        do: {:reply, handle_remove(storage_conf, lease_id), storage_conf}

      def handle_call(:get_all, _from, storage_conf),
        do: {:reply, handle_get_all(storage_conf), storage_conf}
    end
  end
end
