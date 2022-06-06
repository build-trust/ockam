defmodule Ockam.Transport.TCPAddress.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Transport.TCPAddress
  alias Ockam.Address
  alias Ockam.Transport.TCPAddress

  @encoded_localhost <<1, 14, 108, 111, 99, 97, 108, 104, 111, 115, 116, 58, 52, 48, 48, 48>>

  describe "Ockam.Transport.TCPAddress" do
    test "1 is the TCP address type" do
      address = TCPAddress.new({127, 0, 0, 1}, 4000)
      assert 1 == Address.type(address)
    end

    test "can get host and port from address created with host and port" do
      host = "myhost"
      port = 3000

      address = TCPAddress.new(host, port)

      assert {:ok, {^host, ^port}} = TCPAddress.to_host_port(address)
    end

    test "Encoded address produces expected binary" do
      address = TCPAddress.new("localhost", 4000)

      assert {:ok, @encoded_localhost} == Ockam.Wire.encode_address(address)
    end
  end
end
