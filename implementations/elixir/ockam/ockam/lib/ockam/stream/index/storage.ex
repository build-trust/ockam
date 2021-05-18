defmodule Ockam.Stream.Index.Storage do
  @moduledoc false

  @type options() :: Keyword.t()
  @type state() :: any()

  ## TODO: remove stream_name/partition from save and get

  @callback init(options()) :: {:ok, state()} | {:error, any()}
  @callback get_index(
              client_id :: binary(),
              stream_name :: binary(),
              partition :: integer(),
              state()
            ) ::
              {{:ok, non_neg_integer() | :undefnined} | {:error, any()}, state()}
  @callback save_index(
              client_id :: binary(),
              stream_name :: binary(),
              partition :: integer(),
              index :: integer(),
              state()
            ) :: {:ok | {:error, any()}, state()}
end
