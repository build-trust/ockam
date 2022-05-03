defmodule Ockam.Services.TokenLeaseManager do
  @moduledoc false
  use Ockam.Worker

  alias Ockam.Node
  alias Ockam.Services.TokenLeaseManager.Lease

  require Logger

  @impl true
  def setup(options, state) do
    case initialization(options) do
      {:ok, init_opts} -> {:ok, Map.merge(state, init_opts)}
      error -> error
    end
  end

  defp initialization(_options) do
    token_manager_opts = Application.get_env(:ockam_services, :token_manager)
    cloud_service_module = token_manager_opts[:cloud_service_module]
    storage_service_module = token_manager_opts[:storage_service_module]

    with {:ok, _cloud_service_pid} <- get_cloud_service(cloud_service_module),
         {:ok, cloud_service_address} <- cloud_service_module.get_address(),
         {:ok, _storage_service_pid} <-
           get_storage_service(
             storage_service_module,
             cloud_service_module,
             cloud_service_address
           ) do
      # Initializing all saved ttls
      send(self(), :set_all_ttl)

      {:ok,
       %{
         cloud_service: cloud_service_module,
         storage_service: storage_service_module
       }}
    else
      error -> error
    end
  end

  ### for exposing service ##

  @impl true
  def handle_message(%{payload: payload, return_route: return_route}, state) do
    reply_payload = process_request(payload, state)

    Router.route(%{
      payload: reply_payload,
      onward_route: return_route,
      return_route: [state.address]
    })

    {:ok, state}
  end

  ### for self use ##

  @impl true
  def handle_info({:set_ttl, token_id, ttl}, state) do
    set_ttl(self(), token_id, ttl)
    {:noreply, state}
  end

  def handle_info(:set_all_ttl, %{storage_service: storage_service} = state) do
    Logger.info("setting all ttls")
    lease_manager_pid = self()

    Task.start(fn ->
      {:ok, leases} = storage_service.get_all()

      Enum.map(leases, fn lease ->
        ttl = new_ttl(lease.ttl, lease.issued)
        set_ttl(lease_manager_pid, lease.id, ttl)
      end)
    end)

    {:noreply, state}
  end

  def handle_info({:auto_revoke, token_id}, state) do
    Logger.info("revoking #{token_id} token")
    Task.start(fn -> handle_revoke(state[:cloud_service], state[:storage_service], token_id) end)
    {:noreply, state}
  end

  defp process_request(received_payload, state) do
    response =
      case :bare.decode(received_payload, :string) do
        {:ok, payload, _other} -> process_encoded_payload(payload, state)
        _error -> encode_result({:error, "wrong message"})
      end

    :bare.encode(response, :string)
  end

  defp get_cloud_service(mod), do: mod.start_link()

  defp get_storage_service(mod, token_cloud_service, token_cloud_service_address) do
    mod.start_link({to_string(token_cloud_service), token_cloud_service_address})
  end

  defp set_ttl(lease_manager_pid, token_id, ttl) do
    Process.send_after(lease_manager_pid, {:auto_revoke, token_id}, ttl)
  end

  defp handle_create(cloud_service, storage_service, options) do
    {ttl, creation_opts} = Map.pop(options, "ttl")

    case cloud_service.create(creation_opts) do
      {:ok, lease} ->
        updated_lease = %Lease{lease | ttl: ttl}
        storage_service.save(updated_lease)
        set_ttl(self(), updated_lease.id, ttl)
        {:ok, updated_lease}

      {:error, reason} ->
        {:error, reason}
    end
  end

  defp handle_revoke(cloud_service, storage_service, token_id) do
    case cloud_service.revoke(token_id) do
      {:error, reason} ->
        {:error, reason}

      _other ->
        storage_service.remove(token_id)
        :ok
    end
  end

  defp handle_get(cloud_service, storage_service, token_id) do
    case cloud_service.get(token_id) do
      {:ok, _token} ->
        case storage_service.get(token_id) do
          {:ok, token} ->
            {:ok, token}

          _other ->
            cloud_service.revoke(token_id)
            {:error, "token #{token_id} does not exist"}
        end

      :not_found ->
        {:error, "token #{token_id} does not exist"}

      error ->
        error
    end
  end

  defp new_ttl(old_ttl, issued_time) when is_integer(old_ttl) and is_binary(issued_time) do
    case DateTime.from_iso8601(issued_time) do
      {:ok, issued, _utc_offset} ->
        diff = DateTime.diff(DateTime.utc_now(), issued, :millisecond)
        max(old_ttl - diff, 0)

      _error ->
        0
    end
  end

  defp new_ttl(_old_ttl, _issued_time), do: 0

  defp process_encoded_payload(encoded_payload, state) do
    case decode_input(encoded_payload) do
      {:ok, decoded_payload} ->
        process_decoded_payload(decoded_payload, state)

      error ->
        encode_result(error)
    end
  end

  defp process_decoded_payload(payload, state) do
    fun =
      case payload["action"] do
        "create" ->
          fn ->
            handle_create(state[:cloud_service], state[:storage_service], payload["options"])
          end

        "revoke" ->
          fn ->
            handle_revoke(state[:cloud_service], state[:storage_service], payload["token_id"])
          end

        "get" ->
          fn ->
            handle_get(state[:cloud_service], state[:storage_service], payload["token_id"])
          end
      end

    result = fun.()

    encode_result(result)
  end

  defp encode_result(:ok) do
    case Poison.encode(%{result: "success"}) do
      {:ok, data} -> data
      error -> encode_result(error)
    end
  end

  defp encode_result({:ok, lease}) do
    case Poison.encode(%{result: "success", lease: lease}) do
      {:ok, data} -> data
      error -> encode_result(error)
    end
  end

  defp encode_result({:error, error}) do
    case Poison.encode(%{result: "failure", message: error}) do
      {:ok, data} ->
        data

      error ->
        encode_result(error)
    end
  end

  defp decode_input(input) do
    case Poison.decode(input) do
      {:ok, result} ->
        {:ok, result}

      {:error, error} ->
        {:error, error}
    end
  end
end
