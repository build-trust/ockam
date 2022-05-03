defmodule Test.Services.StaticForwardingTest do
  use ExUnit.Case

  alias Ockam.Services.StaticForwarding, as: StaticForwardingService

  alias Ockam.Message
  alias Ockam.Node
  alias Ockam.Router

  test "static forwarding" do
    {:ok, _forwarding, service_address} =
      StaticForwardingService.start_link(
        address: "static_forwarding_service_address",
        prefix: "forward_to"
      )

    {:ok, test_address} = Node.register_random_address()

    alias_str = "test_static_forwarding_alias"

    encoded_alias_str = :bare.encode(alias_str, :string)

    forwarder_address = "forward_to_" <> alias_str

    on_exit(fn ->
      Node.stop(service_address)
      Node.stop(forwarder_address)
      Node.unregister_address(test_address)
    end)

    register_message = %Message{
      onward_route: [service_address],
      payload: encoded_alias_str,
      return_route: [test_address]
    }

    Router.route(register_message)

    assert_receive(
      %Message{
        payload: ^encoded_alias_str,
        onward_route: [^test_address],
        return_route: [^forwarder_address]
      },
      5_000
    )

    forwarded_message = %Message{
      onward_route: [forwarder_address, "smth"],
      payload: "hello",
      return_route: [test_address]
    }

    Router.route(forwarded_message)

    assert_receive(%Message{payload: "hello", onward_route: [^test_address, "smth"]}, 5_000)
  end

  test "forwarding route override" do
    {:ok, _forwarding, service_address} =
      StaticForwardingService.start_link(
        address: "static_forwarding_service_address",
        prefix: "forward_to"
      )

    {:ok, test_address} = Node.register_random_address()

    alias_str = "test_route_override_alias"

    encoded_alias_str = :bare.encode(alias_str, :string)

    forwarder_address = "forward_to_" <> alias_str

    on_exit(fn ->
      Node.stop(service_address)
      Node.stop(forwarder_address)
      Node.unregister_address(test_address)
    end)

    register_message = %Message{
      onward_route: [service_address],
      payload: encoded_alias_str,
      return_route: [test_address]
    }

    Router.route(register_message)

    assert_receive(
      %Message{
        payload: ^encoded_alias_str,
        onward_route: [^test_address],
        return_route: [^forwarder_address]
      },
      5_000
    )

    {:ok, test_address2} = Node.register_random_address()

    register_message2 = %Message{
      onward_route: [service_address],
      payload: encoded_alias_str,
      return_route: [test_address2]
    }

    Router.route(register_message2)

    assert_receive(
      %Message{
        payload: ^encoded_alias_str,
        onward_route: [^test_address2],
        return_route: [^forwarder_address]
      },
      5_000
    )

    forwarded_message = %Message{
      onward_route: [forwarder_address, "smth"],
      payload: "hello",
      return_route: [test_address]
    }

    Router.route(forwarded_message)

    assert_receive(%Message{payload: "hello", onward_route: [^test_address2, "smth"]}, 5_000)

    refute_receive(%Message{payload: "hello", onward_route: [^test_address, "smth"]}, 100)
  end
end
