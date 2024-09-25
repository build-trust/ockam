defmodule Ockam.Transport.Portal.TunnelProtocol.Tests do
  use ExUnit.Case
  doctest Ockam.Transport.Portal.TunnelProtocol
  alias Ockam.Transport.Portal.TunnelProtocol

  describe "Ockam.Transport.Portal.TunnelProtocol" do
    test "protocol msgs can be encoded/decoded" do
      msgs = [
        :ping,
        :pong,
        :disconnect,
        {:payload, {"arbitrary data", :undefined}},
        {:payload, {"arbitrary data", 0xABCD}}
      ]

      assert msgs ==
               msgs
               |> Enum.map(&TunnelProtocol.encode/1)
               |> Enum.map(&TunnelProtocol.decode/1)
               |> Enum.map(fn {:ok, v} -> v end)
    end

    test "format is compatible with rust bare serialization, no packet counter" do
      msg = {:payload, {<<5, 5>>, :undefined}}
      assert <<3, 2, 5, 5, 0>> == TunnelProtocol.encode(msg)
    end

    test "format is compatible with rust bare serialization, with packet counter" do
      msg = {:payload, {<<5, 5>>, 0xAABB}}
      assert <<3, 2, 5, 5, 1, 0xBB, 0xAA>> == TunnelProtocol.encode(msg)
    end

    test "deserialization is compatible with old bare serialization" do
      assert {:ok, {:payload, {<<5, 5>>, :undefined}}} ==
               TunnelProtocol.decode(<<3, 2, 5, 5>>)
    end
  end
end
