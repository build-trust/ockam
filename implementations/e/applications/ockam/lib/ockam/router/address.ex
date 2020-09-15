defprotocol Ockam.Router.Address do
  @moduledoc """
  Defines an elixir protocol for an address that can be in the onward route of
  a message that Ockam.Router can route.

  A address that may be part of a route.
  """

  @dialyzer {:nowarn_function, type: 1}

  @fallback_to_any true

  @typedoc "An integer between 0 and 255 that represents the type of an address."
  @type type :: 0..255 | nil

  @doc """
  Returns the type of an address.
  """
  @spec type(t) :: type()

  def type(address)
end

defimpl Ockam.Router.Address, for: Any do
  import Ockam.Router.Guards

  def type({address_type, _}) when is_address_type(address_type), do: address_type
  def type(_address), do: nil
end
