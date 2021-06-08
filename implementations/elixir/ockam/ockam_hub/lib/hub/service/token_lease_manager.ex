defmodule Ockam.TokenLeaseManager do
  @moduledoc false
  use GenServer

  alias Ockam.TokenLeaseManager.Lease

  @default_cloud_service :influxdb
  @default_storate_service :storage

  def start_link(options) do
    GenServer.start_link(__MODULE__, options, [])
  end

  @impl true
  def init(options) do
    cloud_service_name =
      case Keyword.fetch(options, :cloud_service) do
        {:ok, cloud_service_name} -> cloud_service_name
        _other -> @default_cloud_service
      end

    storage_service_name =
      case Keyword.fetch(options, :storage_service) do
        {:ok, storage_service_name} -> storage_service_name
        _other -> @default_storate_service
      end

    state = %{
      cloud_service: get_cloud_service(cloud_service_name),
      storage_system: get_storage_service(storage_service_name)
    }

    # TODO: initialization for all expirations
    # Process.send(self(), :set_all_expiration, [])

    {:ok, state}
  end

  @impl true
  def handle_info({:create, options}, state) do
    case create(state[:cloud_service], options) do
      {:ok, _lease} -> {:noreply, state}
      error -> error
    end
  end

  def handle_info({:revoke, token_id}, state) do
    case revoke(state[:cloud_service], token_id) do
      :ok -> {:noreply, state}
      error -> error
    end
  end

  def handle_info({:set_expiration, token_id, expiration}, state) do
    set_expiration(self(), token_id, expiration)
    {:noreply, state}
  end

  def handle_info(:set_all_expiration, state) do
    lease_manager_pid = self()

    Task.start(fn ->
      Enum.map(get_all_leases(), fn lease ->
        set_expiration(lease_manager_pid, lease[:token_id], lease[:expiration])
      end)
    end)

    {:noreply, state}
  end

  def handle_info(_message, state) do
    {:noreply, state}
  end

  defp get_cloud_service(name) do
    case name do
      :influxdb -> Ockam.TokenLeaseManager.CloudServiceInfluxdb
      _other -> Ockam.TokenLeaseManager.CloudServiceInfluxdb
    end
  end

  defp get_storage_service(name) do
    # TODO: storage system
    case name do
      _other -> :storage
    end
  end

  defp set_expiration(lease_manager_pid, token_id, expiration) do
    Process.send_after(lease_manager_pid, {:revoke, token_id}, expiration)
  end

  defp save_lease(_lease) do
    # TODO
  end

  defp remove_lease(_tokend_id) do
    # TODO
  end

  defp get_all_leases() do
    # TODO
    []
  end

  defp create(cloud_service, options) do
    {expiration, creation_opts} = Map.pop(options, "expiration")

    case cloud_service.create(creation_opts) do
      {:ok, lease} ->
        save_lease(%Lease{lease | ttl: expiration})
        set_expiration(self(), lease.id, expiration)
        {:ok, lease}

      {:error, reason} ->
        {:error, reason}
    end
  end

  defp revoke(cloud_service, token_id) do
    case cloud_service.revoke(token_id) do
      :ok ->
        remove_lease(token_id)
        :ok

      {:error, reason} ->
        {:error, reason}
    end
  end
end
