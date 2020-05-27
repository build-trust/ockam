defmodule Ockam.Controller.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Controller

  describe "Ockam.Controller" do
    test "responds to ping" do
      Ockam.Router.register("tester", self())
      Ockam.Router.route(0, %Ockam.Message{payload: :ping, return_route: ["tester"]})
      assert_receive %Ockam.Message{payload: :pong}
    end
  end
end
