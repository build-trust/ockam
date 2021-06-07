defmodule Ockam.Node.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Node
  alias Ockam.Node

  describe "#{Node}.register/2" do
    test "can register, send, unregister" do
      Node.register_address("A", self())
      Node.send("A", {:via_node, "hello"})
      assert_receive {:via_node, "hello"}
      Node.unregister_address("A")
    end
  end

  describe "#{Node}.get_random_unregistered_address/{0,1,2}" do
    test "keeps trying" do
      Enum.each(0..254, fn x ->
        x = Base.encode16(<<x>>, case: :lower)
        Node.register_address(x, self())
      end)

      assert "ff" === Node.get_random_unregistered_address("", 1)
    end

    test "uses prefix" do
      assert "prefix" <> _ = Node.get_random_unregistered_address("prefix_")
    end
  end
end
