defmodule Ockam.TokenLeaseManager.CloudService do
  @moduledoc false
  @type lease :: map()
  @type reason :: any()
  @type token_id :: String.t()
  @type options :: Keyword.t()
  @type cloud_configuration :: Keyword.t()
  @type state :: map()
  @type creation_options :: map()

  @callback handle_init(options) :: {:ok, cloud_configuration} | {:error, reason}
  @callback handle_create(cloud_configuration, creation_options) ::
              {:ok, lease} | {:error, reason}
  @callback handle_revoke(cloud_configuration, token_id) :: :ok | {:error, reason}
  @callback handle_renew(cloud_configuration, token_id) :: :ok | {:error, reason}
  @callback handle_get(cloud_configuration, token_id) ::
              {:ok, lease} | :not_found | {:error, reason}
  @callback handle_get_all(cloud_configuration) :: {:ok, [lease]} | {:error, reason}
  @callback handle_get_address(cloud_configuration) :: {:ok, binary()} | {:error, reason}

  defmacro __using__(_opts) do
    quote do
      @behaviour Ockam.TokenLeaseManager.CloudService
      use GenServer

      require Logger

      @name :token_cloud_service

      def start_link(), do: GenServer.start_link(__MODULE__, [], name: @name)
      def create(options), do: GenServer.call(@name, {:create, options})
      def get(lease_id), do: GenServer.call(@name, {:get, lease_id})
      def revoke(lease_id), do: GenServer.call(@name, {:revoke, lease_id})
      def get_all(), do: GenServer.call(@name, :get_all)
      def get_address(), do: GenServer.call(@name, :get_address)

      @impl true
      def init(opts), do: handle_init(opts)

      @impl true
      def handle_call({:create, options}, _from, cloud_configuration) do
        {:reply, handle_create(cloud_configuration, options), cloud_configuration}
      end

      def handle_call({:get, lease_id}, _from, cloud_configuration),
        do: {:reply, handle_get(cloud_configuration, lease_id), cloud_configuration}

      def handle_call({:revoke, lease_id}, _from, cloud_configuration),
        do: {:reply, handle_revoke(cloud_configuration, lease_id), cloud_configuration}

      def handle_call(:get_address, _from, cloud_configuration),
        do: {:reply, handle_get_address(cloud_configuration), cloud_configuration}
    end
  end
end
