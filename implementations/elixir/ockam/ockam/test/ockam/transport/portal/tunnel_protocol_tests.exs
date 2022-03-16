defmodule Ockam.Transport.Portal.TunnelProtocol.Tests do
  use ExUnit.Case
  doctest Ockam.Transport.Portal.TunnelProtocol
  alias Ockam.Transport.Portal.TunnelProtocol

  describe "Ockam.Transport.Portal.TunnelProtocol" do
    test "protocol msgs can be encoded/decoded" do
      msgs = [:ping, :pong, :disconnect, {:payload, "arbitrary data"}]

      assert msgs ==
               msgs
               |> Enum.map(&TunnelProtocol.encode/1)
               |> Enum.map(&TunnelProtocol.decode/1)
               |> Enum.map(fn {:ok, v} -> v end)
    end

    test "format is compatible with rust bare serialization" do
      msg = {:payload, <<5, 5>>}
      assert <<3, 2, 5, 5>> == TunnelProtocol.encode(msg)
    end
  end
end
