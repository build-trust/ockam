defmodule Ockam.Hub.Service.Stream.Storage do
  @moduledoc false

  @type state() :: any()
  @type options() :: Keyword.t()
  @type message() :: %{index: integer(), data: binary()}

  @callback init(String.t(), options()) :: {:ok, state()} | {:error, any()}
  @callback save(String.t(), binary(), state()) :: {{:ok, integer()} | {:error, any()}, state()}
  @callback fetch(String.t(), integer(), integer(), state()) ::
              {{:ok, [message()]} | {:error, any()}, state()}
end
