defmodule Ockam.Protocol.Mapping.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Protocol.Mapping
  alias Ockam.Protocol.Mapping

  alias Ockam.Protocol.Mapping.Tests.Proto1

  describe "Protocol mapping definition" do
    test "Created from server/1" do
      mapping = Mapping.server(Proto1)

      assert is_map(mapping[:in])
      assert is_map(mapping[:out])
      assert is_map(mapping[:modules])

      assert mapping[:in]["proto1"] == Proto1.request()
      assert mapping[:out]["proto1"] == Proto1.response()
    end

    test "Created from client/1" do
    end

    test "Created from mapping/1" do
    end
  end
end
