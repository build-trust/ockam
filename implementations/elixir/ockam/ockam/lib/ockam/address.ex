defprotocol Ockam.RoutableAddress do
  @moduledoc """
  Defines an elixir protocol for an address that can be part of a route.
  """

  @dialyzer {:nowarn_function, type: 1}

  @fallback_to_any true

  @typedoc "An integer between 0 and 255 that represents the type of an address."
  @type type :: 0..255 | atom

  @doc """
  Returns the type of an address.
  """
  @spec type(t) :: type()

  def type(address)

  @doc """
  Returns `true` if `term` is a valid `t:Ockam.Router.Address.type/0`;
  otherwise returns `false`.

  Allowed in guard tests. Inlined by the compiler.
  """
  @doc guard: true
  defguard is_address_type(term)
           when is_atom(term) or (is_integer(term) and term >= 0 and term <= 255)
end

defimpl Ockam.RoutableAddress, for: Any do
  @moduledoc false

  def type(%address_type{}), do: address_type

  def type({address_type, _})
      when is_integer(address_type) and address_type >= 0 and address_type <= 255,
      do: address_type

  def type(_address), do: 0
end
