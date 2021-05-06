defmodule Ockam.Stream.Index.Worker do
  @moduledoc false

  use Ockam.Protocol.Mapping
  use Ockam.Worker

  require Logger

  @default_mod Ockam.Stream.Index.Storage.Internal
  @protocol Ockam.Protocol.Stream.Index
  @partitioned_protocol Ockam.Protocol.Stream.Partitioned.Index

  @protocol_mapping Ockam.Protocol.Mapping.server(Ockam.Protocol.Stream.Index)

  @impl true
  def protocol_mapping() do
    @protocol_mapping
  end

  @impl true
  def setup(options, state) do
    storage_mod = Keyword.get(options, :storage_mod, @default_mod)
    storage_options = Keyword.get(options, :storage_options, [])

    case storage_mod.init(storage_options) do
      {:ok, storage_state} ->
        {:ok, Map.put(state, :storage, {storage_mod, storage_state})}

      {:error, err} ->
        {:error, err}
    end
  end

  @impl true
  def handle_message(%{payload: payload} = message, state) do
    case decode_payload(payload) do
      {:ok, protocol, {:save, data}}
      when protocol == @protocol or protocol == @partitioned_protocol ->
        handle_save(protocol, data, state)

      {:ok, protocol, {:get, data}}
      when protocol == @protocol or protocol == @partitioned_protocol ->
        handle_get(protocol, data, Ockam.Message.return_route(message), state)

      {:error, other} ->
        Logger.error("Unexpected message #{inspect(other)}")
        {:ok, state}
    end
  end

  def handle_save(_protocol, data, state) do
    %{client_id: client_id, stream_name: stream_name, index: index} = data
    partition = Map.get(data, :partition, 0)
    Logger.info("Save index #{inspect({client_id, stream_name, partition, index})}")

    case save_index(client_id, stream_name, partition, index, state) do
      {:ok, state} ->
        {:ok, state}

      {{:error, error}, _state} ->
        Logger.error("Unable to save index: #{inspect(data)}. Reason: #{inspect(error)}")
        {:error, error}
    end
  end

  def handle_get(protocol, data, return_route, state) do
    %{client_id: client_id, stream_name: stream_name} = data
    partition = Map.get(data, :partition, 0)
    Logger.info("get index #{inspect({client_id, stream_name})}")

    case get_index(client_id, stream_name, partition, state) do
      {{:ok, index}, state} ->
        reply_index(protocol, client_id, stream_name, partition, index, return_route, state)
        {:ok, state}

      {{:error, error}, _state} ->
        Logger.error("Unable to get index for: #{inspect(data)}. Reason: #{inspect(error)}")
        {:error, error}
    end
  end

  def save_index(client_id, stream_name, partition, index, state) do
    with_storage(state, fn storage_mod, storage_state ->
      storage_mod.save_index(client_id, stream_name, partition, index, storage_state)
    end)
  end

  def get_index(client_id, stream_name, partition, state) do
    with_storage(state, fn storage_mod, storage_state ->
      storage_mod.get_index(client_id, stream_name, partition, storage_state)
    end)
  end

  def with_storage(state, fun) do
    {storage_mod, storage_state} = Map.get(state, :storage)
    {result, new_storage_state} = fun.(storage_mod, storage_state)
    {result, Map.put(state, :storage, {storage_mod, new_storage_state})}
  end

  def reply_index(protocol, client_id, stream_name, partition, index, return_route, state) do
    Ockam.Router.route(%{
      onward_route: return_route,
      return_route: [state.address],
      payload:
        encode_payload(protocol, %{
          client_id: client_id,
          stream_name: stream_name,
          partition: partition,
          index: index
        })
    })
  end
end
