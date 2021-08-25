defmodule Ockam.Message.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Message
  alias Ockam.Message

  alias Ockam.Transport.TCPAddress
  alias Ockam.Transport.UDPAddress

  describe "Ockam.Router.Message.Any" do
    test "onward_route/1 is empty by default" do
      assert [] === Message.onward_route(:test)
      assert [] === Message.onward_route(100)
      assert [] === Message.onward_route(%{})
      assert [] === Message.onward_route("test")
      assert [] === Message.onward_route([])
      assert [] === Message.onward_route(100.0)
      assert [] === Message.onward_route({100, 300})
    end

    test "onward_route/1 key of map is used if it has a list value" do
      assert [] === Message.onward_route(%{onward_route: []})
      assert [1, 2, 3] === Message.onward_route(%{onward_route: [1, 2, 3]})
    end

    test "onward_route/1 key of map is not used if it does not have a list value" do
      assert [] === Message.onward_route(%{onward_route: 100})
    end

    test "onward_route/1 does what I expect for UDP" do
      udp_address = UDPAddress.new({127, 0, 0, 1}, 3000)

      message = %{
        onward_route: [
          udp_address
        ],
        payload: "hello"
      }

      assert [udp_address] ==
               Message.onward_route(message)
    end

    test "onward_route/1 does what I expect for TCP" do
      tcp_address = TCPAddress.new({127, 0, 0, 1}, 3000)

      message = %{
        onward_route: [
          tcp_address
        ],
        payload: "hello"
      }

      assert [tcp_address] ==
               Message.onward_route(message)
    end

    test "return_route/1 does what I expect for UDP" do
      udp_address = UDPAddress.new({127, 0, 0, 1}, 3000)

      message = %{
        return_route: [
          udp_address
        ],
        payload: "hello"
      }

      assert [^udp_address] = Message.return_route(message)
    end

    test "return_route/1 does what I expect for TCP" do
      tcp_address = TCPAddress.new({127, 0, 0, 1}, 3000)

      message = %{
        return_route: [
          tcp_address
        ],
        payload: "hello"
      }

      assert [^tcp_address] = Message.return_route(message)
    end
  end
end
