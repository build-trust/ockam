defmodule Ockam.Transport.UDPAddress.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Transport.UDPAddress
  alias Ockam.Address
  alias Ockam.Transport.UDPAddress

  @encoded_localhost <<2, 14, 49, 50, 55, 46, 48, 46, 48, 46, 49, 58, 52, 48, 48, 48>>

  describe "Ockam.Transport.UDPAddress" do
    test "2 is the UDP address type" do
      address = UDPAddress.new({127, 0, 0, 1}, 4000)
      assert 2 === Address.type(address)
    end

    test "can get ip and port from address created with ip and port" do
      ip = {127, 0, 0, 1}
      port = 3000

      address = UDPAddress.new(ip, port)

      assert {:ok, {^ip, ^port}} = UDPAddress.to_ip_port(address)
    end

    test "Encoded address produces expected binary" do
      address = UDPAddress.new({127, 0, 0, 1}, 4000)

      assert {:ok, @encoded_localhost} == Ockam.Wire.encode_address(address)
    end
  end
end
