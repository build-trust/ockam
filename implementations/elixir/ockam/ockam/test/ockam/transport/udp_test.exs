defmodule Ockam.Transport.UDP.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Transport.UDP
  alias Ockam.Transport.UDP

  describe "Ockam.Transport.UDP" do
    test "create_listener/1 creates a listener with default options" do
      assert {:ok, _listener} = UDP.create_listener()
    end
  end
end
