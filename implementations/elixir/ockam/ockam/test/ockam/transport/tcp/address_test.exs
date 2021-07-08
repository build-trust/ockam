defmodule Ockam.Transport.TCPAddress.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Transport.TCPAddress
  alias Ockam.Address
  alias Ockam.Transport.TCPAddress

  @tcp 1
  @four_thousand_encoded <<160, 15>>
  @localhost_binary <<0, 127, 0, 0, 1>>
  @hostname_binary <<2, 8, 104, 111, 115, 116, 110, 97, 109, 101, 160, 15>>

  describe "Ockam.Transport.TCPAddress" do
    test "1 is the TCP address type" do
      address = %TCPAddress{host: {127, 0, 0, 1}, port: 4000}
      assert 1 == Address.type(address)
    end

    test "can be serialized and then deserialized back to the original address" do
      address = %TCPAddress{host: {127, 0, 0, 1}, port: 4000}

      serialized = Ockam.Serializable.serialize(address)
      deserialized = TCPAddress.deserialize(serialized)

      assert address === deserialized
    end

    test "Serializing an address produces expected binary" do
      address = %TCPAddress{host: {127, 0, 0, 1}, port: 4000}

      assert %{type: @tcp, value: <<0, 127, 0, 0, 1, 160, 15>>} ==
               Ockam.Serializable.serialize(address)
    end

    test "Deserializing an address produces expected struct" do
      serialized = [@localhost_binary, @four_thousand_encoded]
      assert %TCPAddress{host: {127, 0, 0, 1}, port: 4000} == TCPAddress.deserialize(serialized)
    end

    test "Can serialize and deserialize string hostnames" do
      address = %TCPAddress{host: "hostname", port: 4000}
      serialized = Ockam.Serializable.serialize(address)

      assert %{type: @tcp, value: @hostname_binary} == serialized

      deserialized = TCPAddress.deserialize(serialized)

      assert address == deserialized
    end
  end
end
