defmodule Ockam.Router.Address.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Router.Address
  alias Ockam.Router.Address

  describe "Ockam.Router.Address.Any" do
    test "0 is the default address type" do
      assert nil === Address.type(:test)
      assert nil === Address.type(100)
      assert nil === Address.type(%{})
      assert nil === Address.type("test")
      assert nil === Address.type([])
      assert nil === Address.type(100.0)
    end

    test "first element in two tuple is treated as type if it is an integer" do
      assert 100 === Address.type({100, 200})
    end
  end
end
