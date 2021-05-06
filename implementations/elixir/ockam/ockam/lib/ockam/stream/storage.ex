defmodule Ockam.Stream.Storage do
  @moduledoc false

  @type state() :: any()
  @type options() :: Keyword.t()
  @type message() :: %{index: integer(), data: binary()}

  ## TODO: remove stream_name/partition from save and fetch

  @callback init_stream(String.t(), integer(), options()) :: {:ok, state()} | {:error, any()}
  @callback init_partition(String.t(), integer(), state(), options()) ::
              {:ok, state()} | {:error, any()}

  @callback save(String.t(), integer(), binary(), state()) ::
              {{:ok, integer()} | {:error, any()}, state()}
  @callback fetch(String.t(), integer(), integer(), integer(), state()) ::
              {{:ok, [message()]} | {:error, any()}, state()}
end
