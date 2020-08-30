defprotocol Ockam.Routing.Address do
  @fallback_to_any true

  @type address_type :: 0..255
  @type address_value :: any()

  @spec type(t) :: address_type()
  def type(address)

  @spec value(t) :: address_value()
  def value(address)
end

defimpl Ockam.Routing.Address, for: Any do
  def type({address_type, _address_value})
      when is_integer(address_type) and address_type >= 0 and address_type <= 255,
      do: address_type

  def type(_address), do: Ockam.Routing.local_address_type()

  def value({address_type, address_value})
      when is_integer(address_type) and address_type >= 0 and address_type <= 255,
      do: address_value

  def value(address), do: address
end
