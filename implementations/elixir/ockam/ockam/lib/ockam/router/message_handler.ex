defmodule Ockam.Router.MessageHandler do
  @moduledoc false

  alias Ockam.Message

  @typedoc """
  Ockam message handler.
  Can be a single argument function, that accepts ockam message as an argument
  Or an `{module, function, args}` tuple, which will be called with
  ockam message as a first argument and `args` as the rest

  Should return `:ok` or `{:error, reason}`, where `reason` can be any term.
  """
  @type t :: (Message.t() -> :ok | {:error, reason :: any()}) | {atom(), atom(), [any()]}

  @doc false
  defguard is_mfa(term)
           when is_tuple(term) and tuple_size(term) == 3 and is_atom(elem(term, 0)) and
                  is_atom(elem(term, 1)) and is_list(elem(term, 2))

  @doc """
  Returns `true` if `term` is a valid `t:Ockam.Router.MessageHandler.t/0`;
  otherwise returns `false`.

  Allowed in guard tests. Inlined by the compiler.
  """
  @doc guard: true
  defguard is_message_handler(term) when is_function(term, 1) or is_mfa(term)
end
