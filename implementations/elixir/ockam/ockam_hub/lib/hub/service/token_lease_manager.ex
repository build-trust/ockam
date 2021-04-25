defmodule Ockam.TokenLeaseManager do
  @moduledoc false
  use Ockam.Worker

  alias Ockam.TokenLeaseManager.Lease

  @default_cloud_service :influxdb
  @default_storate_service :storage

  @impl true
  def setup(options, state) do
    cloud_service_name =
      case get_from_options(:cloud_service, options) do
        {:ok, cloud_service_name} -> cloud_service_name
        _ -> @default_cloud_service
      end

    storage_service_name =
      case get_from_options(:storage_service, options) do
        {:ok, storage_service_name} -> storage_service_name
        _ -> @default_storate_service
      end

    state = Map.put(state, :cloud_service, get_cloud_service(cloud_service_name))
    storage = Map.put(state, :storage_system, get_storage_service(storage_service_name))

    #TODO: initialization for all expirations
    # Process.send(self(), :set_all_expiration, [])

    {:ok, state}
  end

  @impl true
  def handle_message({:create, options}, state) do
    case create(state[:cloud_service], options) do
      {:ok, _lease} -> {:ok, state}
      error -> error
    end
  end

  def handle_message({:revoke, token_id}, state) do
    case revoke(state[:cloud_service], token_id) do
      :ok -> {:ok, state}
      error -> error
    end
  end

  def handle_message({:set_expiration, token_id, expiration}, state) do
    set_expiration(self(), token_id, expiration)
    {:ok, state}
  end

  def handle_message(:set_all_expiration, state) do
    lease_manager_pid = self()
    Task.start(
      fn ->
        get_all_leases()
        |> Enum.map(fn lease -> set_expiration(lease_manager_pid, lease[:token_id], lease[:expiration]) end)
      end
    )
    {:ok, state}
  end

  def handle_message(_, state) do
    {:ok, state}
  end


  defp get_cloud_service(name) do
    case name do
      :influxdb -> Ockam.TokenLeaseManager.CloudServiceInfluxdb
      _ -> Ockam.TokenLeaseManager.CloudServiceInfluxdb
    end
  end

  defp get_storage_service(name) do
    # TODO: storage system
    case name do
      _ -> :storage
    end
  end

  defp set_expiration(lease_manager_pid, token_id, expiration) do
    Process.send_after(lease_manager_pid, {:revoke, token_id}, expiration)
  end

  defp save_lease(lease) do
    # TODO
  end

  defp remove_lease(tokend_id) do
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
        :ok
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
