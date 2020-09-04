defmodule Ockam.Router.Guards do
  @moduledoc """
  Defines an elixir macro suitable for use in guard expressions to validate some
  types defined in Ockam.Router.

  See `Ockam.Router`.
  """

  @doc """
  Returns `true` if `term` is a valid `t:Ockam.Router.address_type/0`;
  otherwise returns `false`.

  Allowed in guard tests. Inlined by the compiler.
  """
  defguard is_address_type(term) when is_integer(term) and term >= 0 and term <= 65_535

  defguard is_message_handler(fun) when is_function(fun)
end
