defmodule Ockam.Wire.Binary.Versions.Tests do
  use ExUnit.Case, async: true

  alias Ockam.Transport.TCPAddress
  alias Ockam.Transport.UDPAddress
  alias Ockam.Wire.Binary.Versions

  @localhost <<14, 49, 50, 55, 46, 48, 46, 48, 46, 49>>
  @port_4000 <<58, 52, 48, 48, 48>>
  @port_3000 <<58, 51, 48, 48, 48>>

  describe "Ockam.Wire.Versions" do
    test "encode/1 for TCP" do
      {a, b, c, d} = {127, 0, 0, 1}

      message = %Ockam.Message{
        onward_route: [
          TCPAddress.new({a, b, c, d}, 4000),
          "printer"
        ],
        return_route: [
          TCPAddress.new({a, b, c, d}, 3000)
        ],
        payload: "hello"
      }

      # TODO: embed these in the assert
      _encoded_string_address = <<112, 114, 105, 110, 116, 101, 114>>
      _encoded_payload = <<104, 101, 108, 108, 111>>

      version = 1
      onward_route_size = Enum.count(Map.get(message, :onward_route, []))

      assert {:ok,
              <<^version, ^onward_route_size, 1, @localhost::binary, @port_4000::binary, 0, 7,
                112, 114, 105, 110, 116, 101, 114, 1, 1, @localhost::binary, @port_3000::binary,
                5, 104, 101, 108, 108, 111>>} = Versions.encode(message)
    end

    test "encode/1 for UDP" do
      {a, b, c, d} = {127, 0, 0, 1}

      message = %Ockam.Message{
        onward_route: [
          UDPAddress.new({a, b, c, d}, 4000),
          "printer"
        ],
        return_route: [
          UDPAddress.new({a, b, c, d}, 3000)
        ],
        payload: "hello"
      }

      assert {:ok,
              <<1, 2, 2, @localhost::binary, @port_4000::binary, 0, 7, 112, 114, 105, 110, 116,
                101, 114, 1, 2, @localhost::binary, @port_3000::binary, 5, 104, 101, 108, 108,
                111>>} = Versions.encode(message)
    end

    test "encode/1 for TCP (minimal)" do
      {a, b, c, d} = {127, 0, 0, 1}

      message = %Ockam.Message{
        onward_route: [
          TCPAddress.new({a, b, c, d}, 4000)
        ],
        return_route: [],
        payload: ""
      }

      version = 1
      onward_route_size = 1

      assert {:ok,
              <<^version, ^onward_route_size, 1, @localhost::binary, @port_4000::binary, 0, 0>>} =
               Versions.encode(message)
    end

    test "encode/1 for UDP (minimal)" do
      {a, b, c, d} = {127, 0, 0, 1}

      message = %Ockam.Message{
        onward_route: [
          UDPAddress.new({a, b, c, d}, 4000)
        ],
        return_route: [],
        payload: ""
      }

      assert {:ok, <<1, 1, 2, @localhost::binary, @port_4000::binary, 0, 0>>} =
               Versions.encode(message)
    end

    test "decode/1 for UDP" do
      # TODO: make sure this is valid
      encoded =
        <<1, 2, 2, @localhost::binary, @port_4000::binary, 0, 7, 112, 114, 105, 110, 116, 101,
          114, 1, 2, @localhost::binary, @port_3000::binary, 5, 104, 101, 108, 108, 111>>

      assert {:ok,
              %{
                onward_route: onward_route,
                return_route: return_route,
                payload: payload
              }} = Versions.decode(encoded)

      assert [UDPAddress.new({127, 0, 0, 1}, 4000), "printer"] ==
               onward_route

      assert [UDPAddress.new({127, 0, 0, 1}, 3000)] == return_route
      assert "hello" = payload
    end

    test "decode/1 for TCP" do
      {a, b, c, d} = {127, 0, 0, 1}
      # TODO: make sure this is valid
      # this should always be the same as the output of the TCP serialize test
      encoded = <<1, 1, 1, @localhost::binary, @port_4000::binary, 0, 0>>

      assert {:ok,
              %{
                onward_route: onward_route,
                return_route: return_route,
                payload: payload
              }} = Versions.decode(encoded)

      assert [TCPAddress.new({a, b, c, d}, 4000)] == onward_route
      assert [] = return_route
      assert "" = payload
    end

    test "encode/1 and decode/1 with tracing context" do
      context = "{\"traceparent\":\"00-1234-01\",\"tracestate\":{}}"

      {:ok, encoded} =
        Versions.encode(%Ockam.Message{
          onward_route: ["printer"],
          return_route: [],
          payload: "hello",
          local_metadata: %{tracing_context: context}
        })

      {:ok, decoded} = Versions.decode(encoded)

      assert decoded.local_metadata.tracing_context == context
    end
  end
end
