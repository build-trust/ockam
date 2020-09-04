defmodule Ockam.Router.Message.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Router.Message
  alias Ockam.Router.Message

  describe "Ockam.Router.Message.Any" do
    test "empty onward_route is default" do
      assert [] === Message.onward_route(:test)
      assert [] === Message.onward_route(100)
      assert [] === Message.onward_route(%{})
      assert [] === Message.onward_route("test")
      assert [] === Message.onward_route([])
      assert [] === Message.onward_route(100.0)
      assert [] === Message.onward_route({100, 300})
    end

    test "onward_route key of map is used if it has a list value" do
      assert [] === Message.onward_route(%{onward_route: []})
      assert [1, 2, 3] === Message.onward_route(%{onward_route: [1, 2, 3]})
    end

    test "onward_route key of map is not used if it does not have a list value" do
      assert [] === Message.onward_route(%{onward_route: 100})
    end
  end
end
