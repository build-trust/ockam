defmodule Ockam.Transport.UDPAddress.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Transport.UDPAddress
  alias Ockam.Transport.UDPAddress
  alias Ockam.Address

  @udp 2
  @length_with_port <<7::8>>
  @four_thousand_encoded <<160, 15>>
  @localhost_binary <<0, 127, 0, 0, 1>>

  describe "Ockam.Transport.UDPAddress" do
    test "2 is the UDP address type" do
      address = %UDPAddress{ip: {127, 0, 0, 1}, port: 4000}
      assert 2 === Address.type(address)
    end

    test "can be serialized and then deserialized back to the original address" do
      address = %UDPAddress{ip: {127, 0, 0, 1}, port: 4000}

      serialized = Ockam.Serializable.serialize(address)
      deserialized = UDPAddress.deserialize(serialized)

      assert address === deserialized
    end

    test "Serializing an address produces expected binary" do
      address = %UDPAddress{ip: {127, 0, 0, 1}, port: 4000}
      assert [@udp, @length_with_port, [@localhost_binary, @four_thousand_encoded]] == Ockam.Serializable.serialize(address)
    end

    test "Deserializing an address produces expected struct" do
      serialized = [2, @length_with_port, [@localhost_binary, @four_thousand_encoded]]
      assert %UDPAddress{ip: {127, 0, 0, 1}, port: 4000} == UDPAddress.deserialize(serialized)
    end
  end
end
