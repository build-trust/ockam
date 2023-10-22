defmodule Ockam.Stream.Index.Service do
  @moduledoc false

  use Ockam.Protocol.Mapping
  use Ockam.Worker

  alias Ockam.Stream.Index.Shard

  require Logger

  @default_mod Ockam.Stream.Index.Storage.Internal
  @protocol Ockam.Protocol.Stream.Index
  @partitioned_protocol Ockam.Protocol.Stream.Partitioned.Index

  @protocol_mapping Ockam.Protocol.Mapping.mapping(
                      server: @protocol,
                      server: @partitioned_protocol
                    )

  @impl true
  def protocol_mapping() do
    @protocol_mapping
  end

  @impl true
  def setup(options, state) do
    storage_mod = Keyword.get(options, :storage_mod, @default_mod)
    storage_options = Keyword.get(options, :storage_options, [])

    {:ok, Map.merge(state, %{storage: {storage_mod, storage_options}, shards: %{}})}
  end

  @impl true
  def handle_message(%Ockam.Message{payload: payload} = message, state) do
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
    Logger.debug("Save index #{inspect({client_id, stream_name, partition, index})}")

    shard_id = {client_id, stream_name}
    {shard, state} = ensure_shard(shard_id, state)

    Shard.save_index(shard, partition, index)
    {:ok, state}
  end

  def handle_get(protocol, data, return_route, state) do
    %{client_id: client_id, stream_name: stream_name} = data
    partition = Map.get(data, :partition, 0)
    Logger.debug("get index #{inspect({client_id, stream_name, partition})}")

    shard_id = {client_id, stream_name}
    {shard, state} = ensure_shard(shard_id, state)

    case Shard.get_index(shard, partition) do
      {:ok, index} ->
        reply_index(protocol, client_id, stream_name, partition, index, return_route, state)
        {:ok, state}

      {:error, error} ->
        Logger.error("Unable to get index for: #{inspect(data)}. Reason: #{inspect(error)}")
        {:error, error}
    end
  end

  def ensure_shard(shard_id, state) do
    case find_shard(shard_id, state) do
      {:ok, pid} ->
        {pid, state}

      :error ->
        create_shard(shard_id, state)
    end
  end

  def find_shard(shard_id, state) do
    with {:ok, address} <- Map.fetch(Map.get(state, :shards, %{}), shard_id),
         pid when is_pid(pid) <- Ockam.Node.whereis(address),
         true <- Process.alive?(pid) do
      {:ok, pid}
    else
      _error ->
        :error
    end
  end

  def create_shard(shard_id, state) do
    storage = Map.get(state, :storage)

    {:ok, shard} = Shard.create(shard_id: shard_id, storage: storage)

    shards = Map.get(state, :shards, %{})
    {Ockam.Node.whereis(shard), Map.put(state, :shards, Map.put(shards, shard_id, shard))}
  end

  def reply_index(protocol, client_id, stream_name, partition, index, return_route, state) do
    Ockam.Worker.route(
      %{
        onward_route: return_route,
        return_route: [state.address],
        payload:
          encode_payload(protocol, %{
            client_id: client_id,
            stream_name: stream_name,
            partition: partition,
            index: index
          })
      },
      state
    )
  end
end
