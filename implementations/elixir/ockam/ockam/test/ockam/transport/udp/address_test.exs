defmodule Ockam.Transport.UDPAddress.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Transport.UDPAddress
  alias Ockam.Transport.UDPAddress

  describe "Ockam.Transport.UDPAddress" do
    test "can be serialized and then deserialized back to the original address" do
      address = %UDPAddress{ip: {127, 0, 0, 1}, port: 4000}

      serialized = Ockam.Serializable.serialize(address)
      deserialized = Ockam.Transport.UDPAddress.deserialize(serialized)

      assert address === deserialized
    end
  end
end
