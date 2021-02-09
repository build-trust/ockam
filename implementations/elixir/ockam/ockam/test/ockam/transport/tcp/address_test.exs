defmodule Ockam.Transport.TCPAddress.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Transport.TCPAddress
  alias Ockam.Transport.TCPAddress
  alias Ockam.Address

  @tcp 1
  @length_with_port <<7::8>>
  @four_thousand_encoded <<160, 15>>
  @localhost_binary <<0, 127, 0, 0, 1>>

  describe "Ockam.Transport.TCPAddress" do
    test "1 is the TCP address type" do
      address = %TCPAddress{ip: {127, 0, 0, 1}, port: 4000}
      assert 1 == Address.type(address)
    end

    test "can be serialized and then deserialized back to the original address" do
      address = %TCPAddress{ip: {127, 0, 0, 1}, port: 4000}

      serialized = Ockam.Serializable.serialize(address)
      deserialized = TCPAddress.deserialize(serialized)

      assert address === deserialized
    end

    test "Serializing an address produces expected binary" do
      address = %TCPAddress{ip: {127, 0, 0, 1}, port: 4000}
      assert [@tcp, @length_with_port, [@localhost_binary, @four_thousand_encoded]] == Ockam.Serializable.serialize(address)
    end

    test "Deserializing an address produces expected struct" do
      serialized = [1, @length_with_port, [@localhost_binary, @four_thousand_encoded]]
      assert %TCPAddress{ip: {127, 0, 0, 1}, port: 4000} == TCPAddress.deserialize(serialized)
    end
  end
end
