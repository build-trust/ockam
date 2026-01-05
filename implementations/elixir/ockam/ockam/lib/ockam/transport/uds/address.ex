defmodule Ockam.Transport.UDSAddress do
  alias Ockam.Address

  # uds address type 
  @address_type 3

  def type(), do: @address_type

  def new(path) do
    %Address{type: @address_type, value: path}
  end

  def is_uds_address(address) do
    Address.type(address) == @address_type
  end
end
