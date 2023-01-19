defmodule Ockam.Services.TokenLeaseManager do
  @moduledoc false
  use Ockam.Services.API.Endpoint

  alias Ockam.API.Request
  alias Ockam.Services.TokenLeaseManager.Lease

  require Logger

  @impl true
  def init_endpoint(options) do
    with {:ok, state} <- initialization(options) do
      {:ok, state,
       [
         {:identity, :get, "/", &list/2},
         {:identity, :post, "/", &create_lease/2},
         {:identity, :get, "/:id", &get/2},
         {:identity, :delete, "/:id", &revoke/2}
       ]}
    end
  end

  @impl true
  def authorize(:identity, %Request{} = req, _bindings) do
    case Request.caller_identity_id(req) do
      {:ok, identity_id} ->
        {true, %{identity_id: identity_id}}

      :error ->
        false
    end
  end

  def list(_req, %{
        auth_data: %{identity_id: identity_id},
        state: %{storage_service: storage, storage_service_config: storage_config}
      }) do
    {:ok, leases} = storage.get_all(storage_config, identity_id)
    Logger.info("found #{Enum.count(leases)} leases for identity #{identity_id}")
    Lease.encode_list(leases)
  end

  def create_lease(_req, %{auth_data: %{identity_id: identity_id}, state: state}) do
    %{
      cloud_service: lease_service,
      cloud_service_config: lease_service_config,
      storage_service: storage,
      storage_service_config: storage_service_config,
      ttl: ttl
    } = state

    with {:ok, lease} <- lease_service.create(lease_service_config, identity_id, ttl),
         :ok <- storage.save(storage_service_config, lease) do
      schedule_expire(lease)
      Lease.encode(lease)
    end
  end

  def get(_req, %{bindings: %{id: id}, auth_data: %{identity_id: identity_id}, state: state}) do
    %{storage_service: storage, storage_service_config: storage_service_config} = state

    case storage.get(storage_service_config, identity_id, id) do
      {:ok, %Lease{issued_for: ^identity_id} = lease} ->
        Lease.encode(lease)

      _other ->
        {:error, 404}
    end
  end

  def revoke(_req, %{bindings: %{id: id}, auth_data: %{identity_id: identity_id}, state: state}) do
    with :ok <- do_revoke(state, id, identity_id) do
      {:ok, nil}
    end
  end

  def do_revoke(state, id, identity_id) do
    %{
      cloud_service: lease_service,
      cloud_service_config: lease_service_config,
      storage_service: storage,
      storage_service_config: storage_service_config
    } = state

    case storage.get(storage_service_config, identity_id, id) do
      {:ok, %Lease{issued_for: identity_id}} ->
        with :ok <- lease_service.revoke(lease_service_config, id) do
          storage.remove(storage_service_config, identity_id, id)
        end

      _other ->
        {:error, 404}
    end
  end

  defp initialization(options) do
    {cloud_service_module, cloud_options} = options[:cloud_service]
    {:ok, cloud_service_config} = cloud_service_module.init(cloud_options)
    {:ok, leases} = cloud_service_module.get_all(cloud_service_config)
    Logger.info("Loading #{Enum.count(leases)} leases from backend service")
    storage_service_module = options[:storage_service_module]
    {:ok, storage_service_config} = storage_service_module.init(leases: leases)
    ttl = options[:ttl]
    Enum.each(leases, &schedule_expire/1)

    {:ok,
     %{
       cloud_service: cloud_service_module,
       cloud_service_config: cloud_service_config,
       storage_service: storage_service_module,
       storage_service_config: storage_service_config,
       ttl: ttl
     }}
  end

  def schedule_expire(lease) do
    # TODO: internally it should be datetimes always, serialize when
    #       we need to send it over the wire.
    {:ok, expires, _offset} = DateTime.from_iso8601(lease.expires)
    now = DateTime.utc_now()
    expires_in = max(DateTime.diff(expires, now, :millisecond), 0)
    :timer.send_after(expires_in, {:expire, lease.id, lease.issued_for})
  end

  @impl true
  def handle_info({:expire, lease_id, identity_id}, worker_state) do
    {:ok, api_state} = endpoint_state_from_worker_state(worker_state)
    _ignored = do_revoke(api_state, lease_id, identity_id)
    {:noreply, worker_state}
  end
end
