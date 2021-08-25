defmodule Ockam.Transport.UDP.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Transport.UDP
  alias Ockam.Transport.UDP

  describe "Ockam.Transport.UDP" do
    test "start/1 creates a listener with default options" do
      assert {:ok, listener} = UDP.start()
      socket = listener |> :sys.get_state() |> Map.get(:socket)

      info = Port.info(socket)

      assert 'udp_inet' = Keyword.get(info, :name)
    end
  end
end
