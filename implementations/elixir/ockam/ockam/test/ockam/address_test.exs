defmodule Ockam.Address.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Address
  alias Ockam.Address

  describe "Ockam.Router.Address.Any" do
    test "0 is the default address type" do
      assert 0 === Address.type(:test)
      assert 0 === Address.type(100)
      assert 0 === Address.type(%{})
      assert 0 === Address.type("test")
      assert 0 === Address.type([])
      assert 0 === Address.type(100.0)
    end

    test "first element in two tuple is treated as type if it is an integer" do
      assert 100 === Address.type({100, 200})
    end
  end
end
