defmodule Ockam.Router.MessageHandler do
  @moduledoc false

  alias Ockam.Router.Message

  @typedoc """
  A function that accepts an address and a message as arguments.
  It returns `:ok` or `{:error, reason}`, where `reason` can be any term.
  """
  @type t :: (Message.t() -> :ok | {:error, reason :: any()})
end
