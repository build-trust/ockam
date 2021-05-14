defmodule Ockam.Router.MessageHandler do
  @moduledoc false

  alias Ockam.Routable

  @typedoc """
  A function that accepts an address and a message as arguments.
  It returns `:ok` or `{:error, reason}`, where `reason` can be any term.
  """
  @type t :: (Routable.t() -> :ok | {:error, reason :: any()})

  @doc """
  Returns `true` if `term` is a valid `t:Ockam.Router.MessageHandler.t/0`;
  otherwise returns `false`.

  Allowed in guard tests. Inlined by the compiler.
  """
  @doc guard: true
  defguard is_message_handler(term) when is_function(term)
end
