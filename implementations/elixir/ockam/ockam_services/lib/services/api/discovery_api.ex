defmodule Ockam.Services.API.Discovery do
  @moduledoc """
  API for discovery service

  Methods:

  :get, path: "" - list all services
  :get, path: service_id - get one service info
  :put, path: service_id, body: `Ockam.API.Discovery.ServiceInfo` - register a service
  """

  use Ockam.Services.API

  alias Ockam.API.Discovery.ServiceInfo

  @impl true
  def setup(options, state) do
    storage = Keyword.get(options, :storage, Ockam.Services.Discovery.Storage.Memory)
    storage_options = Keyword.get(options, :storage_options, [])

    {:ok, Map.put(state, :storage, {storage, storage.init(storage_options)})}
  end

  @impl true
  def handle_request(%Request{method: :get, path: ""}, state) do
    case list(state) do
      {infos, state} when is_list(infos) ->
        {:reply, :ok, ServiceInfo.encode_list!(infos), state}

      {:error, _reason} = error ->
        error
    end
  end

  def handle_request(%Request{method: :get, path: id}, state) do
    case get(id, state) do
      {{:ok, info}, state} ->
        {:reply, :ok, ServiceInfo.encode!(info), state}

      {{:error, reason}, state} ->
        {:error, reason, state}
    end
  end

  def handle_request(%Request{method: :put, path: id, body: body, from_route: route}, state) do
    ## Taking route from the from_route instead of service info!
    with {:ok, %{metadata: metadata}} <- ServiceInfo.decode(body) do
      case register(id, route, metadata, state) do
        {:ok, state} ->
          {:reply, :ok, nil, state}

        {{:error, :not_supported}, state} ->
          {:error, :method_not_allowed, state}

        {{:error, reason}, state} ->
          {:error, reason, state}
      end
    end
  end

  def handle_request(%Request{method: _other}, _state) do
    {:error, :method_not_allowed}
  end

  def list(state) do
    with_storage(state, fn storage_mod, storage_state ->
      storage_mod.list(storage_state)
    end)
  end

  def get(id, state) do
    with_storage(state, fn storage_mod, storage_state ->
      storage_mod.get(id, storage_state)
    end)
  end

  def register(id, route, metadata, state) do
    with_storage(state, fn storage_mod, storage_state ->
      storage_mod.register(id, route, metadata, storage_state)
    end)
  end

  def with_storage(state, fun) do
    {storage_mod, storage_state} = Map.get(state, :storage)
    {result, new_storage_state} = fun.(storage_mod, storage_state)
    {result, Map.put(state, :storage, {storage_mod, new_storage_state})}
  end
end
