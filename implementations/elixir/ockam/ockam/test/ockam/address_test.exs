defmodule Ockam.Address.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Address
  alias Ockam.Address

  describe "Ockam.Router.Address.Any" do
    test "0 is the address type for strings" do
      assert 0 === Address.type("test")
    end

    test "type field in address struct is an address type" do
      assert 10 == Address.type(%Address{type: 10})
    end

    test "type is not defined for non addresses" do
      assert_raise(FunctionClauseError, fn -> Address.type(10) end)
    end
  end
end
