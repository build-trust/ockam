defmodule Ockam.Routable.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Routable
  alias Ockam.Routable

  describe "Ockam.Router.Routable.Any" do
    test "onward_route/1 is empty by default" do
      assert [] === Routable.onward_route(:test)
      assert [] === Routable.onward_route(100)
      assert [] === Routable.onward_route(%{})
      assert [] === Routable.onward_route("test")
      assert [] === Routable.onward_route([])
      assert [] === Routable.onward_route(100.0)
      assert [] === Routable.onward_route({100, 300})
    end

    test "onward_route/1 key of map is used if it has a list value" do
      assert [] === Routable.onward_route(%{onward_route: []})
      assert [1, 2, 3] === Routable.onward_route(%{onward_route: [1, 2, 3]})
    end

    test "onward_route/1 key of map is not used if it does not have a list value" do
      assert [] === Routable.onward_route(%{onward_route: 100})
    end

    test "onward_route/1 does what I expect for UDP" do
      message = %{
        onward_route: [
          %Ockam.Transport.UDPAddress{ip: {127, 0, 0, 1}, port: 3000}
        ],
        payload: "hello"
      }

      assert [%Ockam.Transport.UDPAddress{ip: {127, 0, 0, 1}, port: 3000}] ==
               Routable.onward_route(message)
    end

    test "onward_route/1 does what I expect for TCP" do
      message = %{
        onward_route: [
          %Ockam.Transport.TCPAddress{ip: {127, 0, 0, 1}, port: 3000}
        ],
        payload: "hello"
      }

      assert [%Ockam.Transport.TCPAddress{ip: {127, 0, 0, 1}, port: 3000}] ==
               Routable.onward_route(message)
    end

    test "return_route/1 does what I expect for UDP" do
      message = %{
        return_route: [
          %Ockam.Transport.UDPAddress{ip: {127, 0, 0, 1}, port: 3000}
        ],
        payload: "hello"
      }

      assert [%Ockam.Transport.UDPAddress{ip: {127, 0, 0, 1}, port: 3000}] =
               Routable.return_route(message)
    end

    test "return_route/1 does what I expect for TCP" do
      message = %{
        return_route: [
          %Ockam.Transport.TCPAddress{ip: {127, 0, 0, 1}, port: 3000}
        ],
        payload: "hello"
      }

      assert [%Ockam.Transport.TCPAddress{ip: {127, 0, 0, 1}, port: 3000}] =
               Routable.return_route(message)
    end
  end
end
