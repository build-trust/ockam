defmodule Test.Services.ForwardingTest do
  use ExUnit.Case

  alias Ockam.Services.Forwarding, as: ForwardingService

  alias Ockam.Node
  alias Ockam.Router
  alias Ockam.Workers.RemoteForwarder

  test "forwarding" do
    {:ok, _service_pid, service_address} =
      ForwardingService.start_link(address: "forwarding_address")

    {:ok, me} = Node.register_random_address()

    register_message = %Ockam.Message{
      onward_route: [service_address],
      return_route: [me],
      payload: ""
    }

    Router.route(register_message)

    assert_receive(%{onward_route: [^me], return_route: forwarder_route}, 5_000)

    forwarder_address = List.last(forwarder_route)

    on_exit(fn ->
      Node.stop(service_address)
      Node.stop(forwarder_address)
      Node.unregister_address(me)
    end)

    msg = %{onward_route: [forwarder_address], return_route: [me], payload: "HI!"}

    Router.route(msg)

    assert_receive(%{onward_route: [^me], return_route: reply_route, payload: "HI!"}, 5_000)

    assert me == List.last(reply_route)
  end

  test "forwarding with onward route" do
    {:ok, _forwarding, service_address} =
      ForwardingService.start_link(address: "forwarding_address")

    {:ok, me} = Node.register_random_address()

    {:ok, forwarder} = RemoteForwarder.create(service_route: [service_address], forward_to: [me])

    forwarder_address = RemoteForwarder.forwarder_address(forwarder)

    on_exit(fn ->
      Node.stop(service_address)
      Node.stop(forwarder_address)
      Node.unregister_address(me)
    end)

    msg = %{onward_route: [forwarder_address, "foo"], payload: "HI", return_route: [me]}

    Router.route(msg)

    assert_receive(%{onward_route: [^me, "foo"]}, 5_000)
  end
end
