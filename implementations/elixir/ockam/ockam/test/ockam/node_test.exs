defmodule Ockam.Node.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Node
  alias Ockam.Node

  describe "#{Node}.register/2" do
    test "can register, send, unregister" do
      Node.register("A", self())
      Node.send("A", {:via_node, "hello"})
      assert_receive {:via_node, "hello"}
      Node.unregister("A")
    end
  end

  describe "#{Node}.get_unused_address/{0,1}" do
    test "keeps trying" do
      Enum.each(0..254, fn x -> Node.register(<<x>>, self()) end)
      assert <<255>> === Node.get_unused_address(1)
    end
  end
end
