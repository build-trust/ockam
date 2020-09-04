defprotocol Ockam.Router.Address do
  @moduledoc """
  Defines an elixir protocol for an address that Ockam.Router can handle.
  """

  @fallback_to_any true

  @typedoc "An integer between 0 and 65535 that represents the type of an address."
  @type type :: 0..65_535

  @typedoc ""
  @type value :: any()

  @spec type(t) :: type()
  def type(address)

  @spec value(t) :: value()
  def value(address)
end

defimpl Ockam.Router.Address, for: Any do
  import Ockam.Router.Guards

  def type({address_type, _address_value}) when is_address_type(address_type), do: address_type
  def type(_address), do: 0

  def value({address_type, address_value}) when is_address_type(address_type), do: address_value
  def value(address), do: address
end
