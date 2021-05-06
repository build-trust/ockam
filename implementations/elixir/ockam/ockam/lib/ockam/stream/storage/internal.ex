defmodule Ockam.Stream.Storage.Internal do
  @moduledoc false
  @behaviour Ockam.Stream.Storage

  @type message() :: Ockam.Stream.Storage.message()

  @type storage() :: map()

  @spec init_stream(String.t(), integer(), list()) :: {:ok, storage()} | {:error, any()}
  def init_stream(_stream_name, _partitions, _options) do
    {:ok, nil}
  end

  @spec init_partition(String.t(), integer(), any(), list()) :: {:ok, storage()} | {:error, any()}
  def init_partition(_stream_name, _partition, _stream_state, _options) do
    {:ok, %{latest: 0, earliest: 0}}
  end

  @spec save(String.t(), integer(), binary(), storage()) ::
          {{:ok, integer()} | {:error, any()}, storage()}
  def save(_stream_name, _partition, data, storage) do
    latest = Map.get(storage, :latest, 0)
    next = latest + 1
    message = %{index: next, data: data}

    new_storage =
      storage
      |> Map.put(next, message)
      |> Map.put(:latest, next)

    {{:ok, next}, new_storage}
  end

  @spec fetch(String.t(), integer(), integer(), integer(), storage()) ::
          {{:ok, [message()]} | {:error, any()}, storage()}
  def fetch(_stream_name, _partition, index, limit, storage) do
    earliest = Map.get(storage, :earliest, 0)
    start_from = max(index, earliest)
    end_on = start_from + limit - 1

    ## Naive impl. Gaps are ignored as there shouldn't be any
    messages =
      :lists.seq(start_from, end_on)
      |> Enum.map(fn i -> Map.get(storage, i) end)
      |> Enum.reject(&is_nil/1)

    {{:ok, messages}, storage}
  end
end
