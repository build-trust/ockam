defmodule Ockam.Wire.Binary.V2.Tests do
  use ExUnit.Case, async: true

  alias Ockam.Transport.TCPAddress
  alias Ockam.Transport.UDPAddress
  alias Ockam.Wire.Binary.V2

  describe "Ockam.Wire.V2" do
    test "encode/1 for TCP" do
      {a, b, c, d} = {127, 0, 0, 1}

      message = %{
        onward_route: [
          %TCPAddress{ip: {a, b, c, d}, port: 4000},
          "printer"
        ],
        return_route: [
          %TCPAddress{ip: {a, b, c, d}, port: 3000}
        ],
        payload: "hello"
      }

      # TODO: embed these in the assert
      _encoded_string_address = <<112, 114, 105, 110, 116, 101, 114>>
      _encoded_payload = <<104, 101, 108, 108, 111>>

      version = 1
      onward_route_size = Enum.count(Map.get(message, :onward_route, []))

      assert {:ok,
              <<^version, ^onward_route_size, 1, 7, 0, 127, 0, 0, 1, 160, 15, 0, 7, 112, 114, 105,
                110, 116, 101, 114, 1, 1, 7, 0, 127, 0, 0, 1, 184, 11, 5, 104, 101, 108, 108,
                111>>} = V2.encode(message)
    end

    test "encode/1 for UDP" do
      {a, b, c, d} = {127, 0, 0, 1}

      message = %{
        onward_route: [
          %UDPAddress{ip: {a, b, c, d}, port: 4000},
          "printer"
        ],
        return_route: [
          %UDPAddress{ip: {a, b, c, d}, port: 3000}
        ],
        payload: "hello"
      }

      assert {:ok,
              <<1, 2, 2, 7, 0, 127, 0, 0, 1, 160, 15, 0, 7, 112, 114, 105, 110, 116, 101, 114, 1,
                2, 7, 0, 127, 0, 0, 1, 184, 11, 5, 104, 101, 108, 108, 111>>} = V2.encode(message)
    end

    test "encode/1 for TCP (minimal)" do
      {a, b, c, d} = {127, 0, 0, 1}

      message = %{
        onward_route: [
          %TCPAddress{ip: {a, b, c, d}, port: 4000}
        ],
        return_route: [],
        payload: ""
      }

      version = 1
      onward_route_size = 1

      assert {:ok, <<^version, ^onward_route_size, 1, 7, 0, ^a, ^b, ^c, ^d, 160, 15, 0, 0>>} =
               V2.encode(message)
    end

    test "encode/1 for UDP (minimal)" do
      {a, b, c, d} = {127, 0, 0, 1}

      message = %{
        onward_route: [
          %UDPAddress{ip: {a, b, c, d}, port: 4000}
        ],
        return_route: [],
        payload: ""
      }

      assert {:ok, <<1, 1, 2, 7, 0, 127, 0, 0, 1, 160, 15, 0, 0>>} = V2.encode(message)
    end

    test "decode/1 for UDP" do
      # TODO: make sure this is valid
      encoded =
        <<1, 2, 2, 7, 0, 127, 0, 0, 1, 160, 15, 0, 7, 112, 114, 105, 110, 116, 101, 114, 1, 2, 7,
          0, 127, 0, 0, 1, 184, 11, 5, 104, 101, 108, 108, 111>>

      assert {:ok,
              %{
                onward_route: onward_route,
                return_route: return_route,
                payload: payload
              }} = V2.decode(encoded)

      assert [%Ockam.Transport.UDPAddress{ip: {127, 0, 0, 1}, port: 4000}, "printer"] =
               onward_route

      assert [%Ockam.Transport.UDPAddress{ip: {127, 0, 0, 1}, port: 3000}] = return_route
      assert "hello" = payload
    end

    test "decode/1 for TCP" do
      {a, b, c, d} = {127, 0, 0, 1}
      # TODO: make sure this is valid
      # this should always be the same as the output of the TCP serialize test
      encoded = <<1, 1, 1, 7, 0, a, b, c, d, 160, 15, 0, 0>>

      assert {:ok,
              %{
                onward_route: onward_route,
                return_route: return_route,
                payload: payload
              }} = V2.decode(encoded)

      assert [%Ockam.Transport.TCPAddress{ip: {^a, ^b, ^c, ^d}, port: 4000}] = onward_route
      assert [] = return_route
      assert "" = payload
    end
  end
end
