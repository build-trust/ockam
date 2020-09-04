defmodule Ockam.Router.Address.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Router.Address
  alias Ockam.Router.Address

  describe "Ockam.Router.Address.Any" do
    test "0 is the default address type" do
      assert 0 === Address.type(:test)
      assert 0 === Address.type(100)
      assert 0 === Address.type(%{})
      assert 0 === Address.type("test")
      assert 0 === Address.type([])
      assert 0 === Address.type(100.0)
    end

    test "default address value is same as input" do
      assert :test === Address.value(:test)
      assert 100 === Address.value(100)
      assert %{} === Address.value(%{})
      assert "test" === Address.value("test")
      assert [] === Address.value([])
      assert 100.0 === Address.value(100.0)
    end

    test "first element in two tuple is treated as type if it is an integer" do
      assert 100 === Address.type({100, 200})
    end

    test "second element in two tuple is treated as value if first element is an integer" do
      assert 200 === Address.value({100, 200})
    end
  end
end
