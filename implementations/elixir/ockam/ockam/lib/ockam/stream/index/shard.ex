defmodule Ockam.Stream.Index.Shard do
  @moduledoc """
  Stream index management shard.
  This module performs storage operations and keeping state per client_id/stream_name pair
  """

  use GenServer

  require Logger

  def start_link(shard_id, storage) do
    GenServer.start_link(__MODULE__, [shard_id, storage])
  end

  @impl true
  def init([{client_id, stream_name}, {storage_mod, storage_options}]) do
    case storage_mod.init(storage_options) do
      {:ok, storage_state} ->
        {:ok,
         %{
           storage: {storage_mod, storage_state},
           client_id: client_id,
           stream_name: stream_name
         }}

      {:error, error} ->
        Logger.error("Stream index setup error: #{inspect(error)}")
        {:error, error}
    end
  end

  @impl true
  def handle_cast({:save_index, partition, index}, state) do
    %{client_id: client_id, stream_name: stream_name} = state

    case save_index(client_id, stream_name, partition, index, state) do
      {:ok, state} ->
        {:noreply, state}

      {{:error, error}, state} ->
        Logger.error(
          "Unable to save index: #{inspect({client_id, stream_name, partition, index})}. Reason: #{
            inspect(error)
          }"
        )

        {:stop, :normal, state}
    end
  end

  @impl true
  def handle_call({:get_index, partition}, _from, state) do
    %{client_id: client_id, stream_name: stream_name} = state

    case get_index(client_id, stream_name, partition, state) do
      {{:ok, index}, state} ->
        {:reply, {:ok, index}, state}

      {{:error, error}, state} ->
        Logger.error(
          "Unable to get index for: #{inspect({client_id, stream_name, partition})}. Reason: #{
            inspect(error)
          }"
        )

        {:reply, {:error, error}, state}
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
end
