defmodule Test.Services.StaticForwardingTest do
  use ExUnit.Case

  # Fail after 200ms of retrying with time between attempts 10ms
  use AssertEventually, timeout: 200, interval: 10

  alias Ockam.Identity
  alias Ockam.SecureChannel

  alias Ockam.Services.Relay.StaticForwarding, as: StaticForwardingService
  alias Ockam.Services.Relay.Types.CreateRelayRequest
  alias Ockam.Services.Relay.Types.Relay

  alias Ockam.Message
  alias Ockam.Node
  alias Ockam.Router

  test "static forwarding" do
    {:ok, service_address} = StaticForwardingService.create(prefix: "forward_to")

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

    assert_eventually(
      [%Relay{addr: ^forwarder_address, target_identifier: nil}] =
        StaticForwardingService.list_running_relays()
    )

    Ockam.Node.stop(forwarder_address)
    assert_eventually([] = StaticForwardingService.list_running_relays())
  end

  test "static forwarding cbor msg with tags" do
    {:ok, service_address} = StaticForwardingService.create(prefix: "forward_to")

    {:ok, test_address} = Node.register_random_address()

    alias_str = "test_static_forwarding_alias"

    forwarder_address = "forward_to_" <> alias_str

    on_exit(fn ->
      Node.stop(service_address)
      Node.stop(forwarder_address)
      Node.unregister_address(test_address)
    end)

    req = %CreateRelayRequest{alias: alias_str, tags: %{"name" => "test"}}

    register_message = %Message{
      onward_route: [service_address],
      payload: CreateRelayRequest.encode!(req),
      return_route: [test_address]
    }

    Router.route(register_message)

    assert_receive(
      %Message{
        payload: encoded_payload,
        onward_route: [^test_address],
        return_route: [^forwarder_address]
      },
      5_000
    )

    assert {:ok, alias_str, ""} == :bare.decode(encoded_payload, :string)

    forwarded_message = %Message{
      onward_route: [forwarder_address, "smth"],
      payload: "hello",
      return_route: [test_address]
    }

    Router.route(forwarded_message)

    assert_receive(%Message{payload: "hello", onward_route: [^test_address, "smth"]}, 5_000)

    assert_eventually(
      [%Relay{addr: ^forwarder_address, target_identifier: nil, tags: %{"name" => "test"}}] =
        StaticForwardingService.list_running_relays()
    )

    Ockam.Node.stop(forwarder_address)
    assert_eventually([] = StaticForwardingService.list_running_relays())
  end

  test "forwarding route override" do
    {:ok, listener_identity} = Identity.create()
    {:ok, listener_keypair} = SecureChannel.Crypto.generate_dh_keypair()
    {:ok, attestation} = Identity.attest_purpose_key(listener_identity, listener_keypair)

    {:ok, listener} =
      SecureChannel.create_listener(
        identity: listener_identity,
        encryption_options: [
          static_keypair: listener_keypair,
          static_key_attestation: attestation
        ]
      )

    {:ok, bob} = Identity.create()
    {:ok, alice} = Identity.create()
    bob_id = Identity.get_identifier(bob)
    {:ok, bob_keypair} = SecureChannel.Crypto.generate_dh_keypair()
    {:ok, bob_attestation} = Identity.attest_purpose_key(bob, bob_keypair)
    {:ok, alice_keypair} = SecureChannel.Crypto.generate_dh_keypair()
    {:ok, alice_attestation} = Identity.attest_purpose_key(alice, alice_keypair)

    {:ok, bob_channel} =
      SecureChannel.create_channel(
        identity: bob,
        encryption_options: [static_keypair: bob_keypair, static_key_attestation: bob_attestation],
        route: [listener]
      )

    {:ok, alice_channel} =
      SecureChannel.create_channel(
        identity: alice,
        encryption_options: [
          static_keypair: alice_keypair,
          static_key_attestation: alice_attestation
        ],
        route: [listener]
      )

    {:ok, service_address} = StaticForwardingService.create(prefix: "forward_to")

    {:ok, test_address} = Node.register_random_address()

    alias_str = "test_route_override_alias"

    encoded_alias_str = :bare.encode(alias_str, :string)

    forwarder_address = "forward_to_" <> alias_str

    on_exit(fn ->
      Node.stop(service_address)
      Node.stop(forwarder_address)
      Node.unregister_address(test_address)
    end)

    # Bob creates the relay
    register_message = %Message{
      onward_route: [bob_channel, service_address],
      payload: encoded_alias_str,
      return_route: [test_address]
    }

    Router.route(register_message)

    assert_receive(
      %Message{
        payload: ^encoded_alias_str,
        onward_route: [^test_address],
        return_route: [^bob_channel, ^forwarder_address]
      },
      5_000
    )

    assert_eventually(
      [
        %Relay{
          addr: ^forwarder_address,
          created_at: t1,
          updated_at: t2,
          target_identifier: ^bob_id
        }
      ] = StaticForwardingService.list_running_relays()
    )

    assert t1 == t2

    {:ok, test_address2} = Node.register_random_address()

    register_message2 = %Message{
      onward_route: [bob_channel, service_address],
      payload: encoded_alias_str,
      return_route: [test_address2]
    }

    # Bob make a modification to the relay, it's allowed
    Router.route(register_message2)

    assert_receive(
      %Message{
        payload: ^encoded_alias_str,
        onward_route: [^test_address2],
        return_route: [^bob_channel, ^forwarder_address]
      },
      5_000
    )

    assert_eventually(
      (
        [%Relay{addr: ^forwarder_address, created_at: ^t1, updated_at: t3}] =
          StaticForwardingService.list_running_relays()

        :lt == DateTime.compare(t1, t3)
      )
    )

    forwarded_message = %Message{
      onward_route: [forwarder_address, "smth"],
      payload: "hello",
      return_route: [test_address]
    }

    Router.route(forwarded_message)

    assert_receive(%Message{payload: "hello", onward_route: [^test_address2, "smth"]}, 5_000)

    refute_receive(%Message{payload: "hello", onward_route: [^test_address, "smth"]}, 100)

    {:ok, test_address3} = Node.register_random_address()

    register_message2 = %Message{
      onward_route: [alice_channel, service_address],
      payload: encoded_alias_str,
      return_route: [test_address3]
    }

    # Alice try to make a modification to the relay, it isn't allowed
    Router.route(register_message2)
    refute_receive(%Message{onward_route: [^test_address3]}, 100)

    # Relay is unchanged
    Router.route(forwarded_message)
    assert_receive(%Message{payload: "hello", onward_route: [^test_address2, "smth"]}, 5_000)
  end
end
