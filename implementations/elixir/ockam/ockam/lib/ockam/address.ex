defprotocol Ockam.Address do
  @moduledoc """
  Defines an elixir protocol for an address that can be part of a route.
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

  @doc """
  Returns the value of an address.
  """
  @spec value(t) :: String.Chars.t()

  def value(address)
end

defimpl Ockam.Address, for: Any do
  @moduledoc false

  def type({address_type, _})
      when is_integer(address_type) and address_type >= 0 and address_type <= 255,
      do: address_type

  def type(_address), do: nil

  def value({address_type, address_value})
      when is_integer(address_type) and address_type >= 0 and address_type <= 255,
      do: address_value

  def value(address), do: address
end
