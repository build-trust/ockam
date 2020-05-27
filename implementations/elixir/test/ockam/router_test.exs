defmodule Ockam.Router.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Router

  describe "Ockam.Router.register/2" do
    test "can register and send" do
      Ockam.Router.register("A", self())
      Ockam.Router.route("A", {:via_router, "hello"})
      assert_receive {:via_router, "hello"}
      Ockam.Router.unregister("A")
    end
  end

  describe "Ockam.Router.get_unused_address/{0,1}" do
    test "keeps trying" do
      Enum.each(0..254, fn x ->
        <<x>> |> Base.encode16(case: :lower) |> Ockam.Router.register(self())
      end)

      assert Base.encode16(<<255>>, case: :lower) === Ockam.Router.get_unused_address(1)

      Enum.each(0..255, fn x ->
        <<x>> |> Base.encode16(case: :lower) |> Ockam.Router.register(self())
      end)
    end
  end
end
