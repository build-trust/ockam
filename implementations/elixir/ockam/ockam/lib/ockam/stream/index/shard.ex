defmodule Ockam.Stream.Index.Shard do
  @moduledoc """
  Stream index management shard.
  This module performs storage operations and keeping state per client_id/stream_name pair
  """

  use Ockam.Worker

  require Logger

  def get_index(shard, partition) do
    GenServer.call(shard, {:get_index, partition})
  end

  def save_index(shard, partition, index) do
    GenServer.cast(shard, {:save_index, partition, index})
  end

  @impl true
  def setup(options, state) do
    {:ok, {client_id, stream_name}} = Keyword.fetch(options, :shard_id)
    {:ok, {storage_mod, storage_options}} = Keyword.fetch(options, :storage)

    case storage_mod.init(storage_options) do
      {:ok, storage_state} ->
        {:ok,
         Map.merge(
           state,
           %{
             storage: {storage_mod, storage_state},
             client_id: client_id,
             stream_name: stream_name
           }
         )}

      {:error, error} ->
        Logger.error("Stream index setup error: #{inspect(error)}")
        {:error, error}
    end
  end

  @impl true
  def handle_message(_msg, state) do
    {:ok, state}
  end

  @impl true
  def handle_cast({:save_index, partition, index}, state) do
    %{client_id: client_id, stream_name: stream_name} = state

    case save_index(client_id, stream_name, partition, index, state) do
      {:ok, state} ->
        {:noreply, update_ts(state)}

      {{:error, error}, state} ->
        Logger.error(
          "Unable to save index: #{inspect({client_id, stream_name, partition, index})}. Reason: #{inspect(error)}"
        )

        {:stop, :normal, state}
    end
  end

  @impl true
  def handle_call({:get_index, partition}, _from, state) do
    %{client_id: client_id, stream_name: stream_name} = state

    case get_index(client_id, stream_name, partition, state) do
      {{:ok, index}, state} ->
        {:reply, {:ok, index}, update_ts(state)}

      {{:error, error}, state} ->
        Logger.error(
          "Unable to get index for: #{inspect({client_id, stream_name, partition})}. Reason: #{inspect(error)}"
        )

        {:reply, {:error, error}, update_ts(state)}
    end
  end

  def update_ts(state) do
    Map.put(state, :last_message_ts, System.os_time(:millisecond))
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
end
