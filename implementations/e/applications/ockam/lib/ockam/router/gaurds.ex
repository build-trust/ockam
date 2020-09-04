defmodule Ockam.Router.Guards do
  @moduledoc """
  Defines elixir macros suitable for use in guard expressions to validate
  some types used by Ockam.Router.
  """

  @doc """
  Returns `true` if `term` is a valid `t:Ockam.Router.Address.type/0`;
  otherwise returns `false`.

  Allowed in guard tests. Inlined by the compiler.
  """
  @doc guard: true
  defguard is_address_type(term) when is_integer(term) and term >= 0 and term <= 65_535

  @doc """
  Returns `true` if `term` is a valid `t:Ockam.Router.Message.t/0`;
  otherwise returns `false`.

  Allowed in guard tests. Inlined by the compiler.
  """
  @doc guard: true
  defguard is_message_handler(term) when is_function(term)
end
