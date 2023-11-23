defmodule Test.Services.StaticForwardingTest do
  use ExUnit.Case

  # Fail after 200ms of retrying with time between attempts 10ms
  use AssertEventually, timeout: 200, interval: 10

  alias Ockam.Credential.AttributeStorageETS, as: AttributeStorage

  alias Ockam.Identity
  alias Ockam.SecureChannel

  alias Ockam.Services.Relay.StaticForwarding, as: StaticForwardingService
  alias Ockam.Services.Relay.Types.CreateRelayRequest
  alias Ockam.Services.Relay.Types.Relay

  alias Ockam.Message
  alias Ockam.Node
  alias Ockam.Router

  setup do
    {:ok, alice} = Identity.create()
    {:ok, bob} = Identity.create()
    {:ok, carol} = Identity.create()
    {:ok, authority} = Identity.create()

    {:ok, listener_identity} = Identity.create()
    {:ok, listener_keypair} = SecureChannel.Crypto.generate_dh_keypair()
    {:ok, attestation} = Identity.attest_purpose_key(listener_identity, listener_keypair)

    {:ok, listener} =
      SecureChannel.create_listener(
        identity: listener_identity,
        encryption_options: [
          static_keypair: listener_keypair,
          static_key_attestation: attestation
        ],
        authorities: [authority]
      )

    # TODO: rework the relationship on credential exchange API, attribute storage and secure channel
    :ok = AttributeStorage.init()
    {:ok, service_address} = StaticForwardingService.create(prefix: "forward_to")

    on_exit(fn ->
      :ok = Node.stop(service_address)
      :ok = Node.stop(listener)
    end)

    {:ok,
     authority: authority,
     alice: alice,
     bob: bob,
     carol: carol,
     listener: listener,
     service_addr: service_address}
  end

  defp create_channel_with_credential(authority, identity, listener, attributes) do
    {:ok, credential} =
      Identity.issue_credential(authority, Identity.get_identifier(identity), attributes, 100)

    {:ok, keypair} = SecureChannel.Crypto.generate_dh_keypair()
    {:ok, attestation} = Identity.attest_purpose_key(identity, keypair)

    SecureChannel.create_channel(
      identity: identity,
      encryption_options: [static_keypair: keypair, static_key_attestation: attestation],
      route: [listener],
      authorities: [authority],
      credentials: [credential]
    )
  end

  defp register_relay(channel, service_address, payload, return_route) do
    register_message = %Message{
      onward_route: [channel, service_address],
      payload: payload,
      return_route: return_route
    }

    Router.route(register_message)
  end

  defp assert_register_relay(channel, service_address, alias_str, return_route, tags \\ nil) do
    forwarder_address = "forward_to_" <> alias_str
    encoded_alias_str = :bare.encode(alias_str, :string)

    payload =
      case tags do
        nil ->
          encoded_alias_str

        %{} ->
          req = %CreateRelayRequest{alias: alias_str, tags: tags}
          CreateRelayRequest.encode!(req)
      end

    register_relay(channel, service_address, payload, return_route)

    assert_receive(
      %Message{
        payload: ^encoded_alias_str,
        onward_route: ^return_route,
        return_route: [^channel, ^forwarder_address]
      },
      5_000
    )

    {:ok, forwarder_address}
  end

  defp refute_register_relay(channel, service_address, alias_str, return_route) do
    encoded_alias_str = :bare.encode(alias_str, :string)
    register_relay(channel, service_address, encoded_alias_str, return_route)
    refute_receive(%Message{onward_route: ^return_route}, 200)
    :ok
  end

  defp assert_message_pass_through_relay(forwarder_address, registered_route) do
    # Messages sent to the relay are delivered to alice
    forwarded_message = %Message{
      onward_route: [forwarder_address, "smth"],
      payload: "hello",
      return_route: []
    }

    Router.route(forwarded_message)
    expected_route = registered_route ++ ["smth"]
    assert_receive(%Message{payload: "hello", onward_route: ^expected_route}, 5_000)
  end

  test "static forwarding", %{
    authority: authority,
    alice: alice,
    bob: bob,
    carol: carol,
    listener: listener,
    service_addr: service_address
  } do
    {:ok, test_address_alice} = Node.register_random_address()
    {:ok, test_address_bob} = Node.register_random_address()
    {:ok, test_address_carol} = Node.register_random_address()

    alias_str = "test_static_forwarding_alias"

    forwarder_address = "forward_to_" <> alias_str

    on_exit(fn ->
      Node.stop(forwarder_address)
      Node.unregister_address(test_address_alice)
      Node.unregister_address(test_address_bob)
    end)

    {:ok, channel_alice} =
      create_channel_with_credential(authority, alice, listener, %{
        "allow_relay_address" => alias_str
      })

    {:ok, channel_bob} =
      create_channel_with_credential(authority, bob, listener, %{"allow_relay_address" => "*"})

    {:ok, channel_carol} =
      create_channel_with_credential(authority, carol, listener, %{
        "allow_relay_address" => "other"
      })

    {:ok, ^forwarder_address} =
      assert_register_relay(channel_alice, service_address, alias_str, [test_address_alice])

    assert_message_pass_through_relay(forwarder_address, [test_address_alice])

    # Metadata on the relay point to alice
    alice_id = Identity.get_identifier(alice)

    assert_eventually(
      [%Relay{addr: ^forwarder_address, target_identifier: ^alice_id}] =
        StaticForwardingService.list_running_relays()
    )

    # Bob can take the relay. It also shows that can attach tags to it
    bob_tags = %{"some" => "tag"}

    {:ok, ^forwarder_address} =
      assert_register_relay(channel_bob, service_address, alias_str, [test_address_bob], bob_tags)

    assert_message_pass_through_relay(forwarder_address, [test_address_bob])

    # Metadata on the relay point to bob
    bob_id = Identity.get_identifier(bob)

    assert_eventually(
      [%Relay{addr: ^forwarder_address, target_identifier: ^bob_id, tags: ^bob_tags}] =
        StaticForwardingService.list_running_relays()
    )

    # Carol can't register on this address
    :ok = refute_register_relay(channel_carol, service_address, alias_str, [test_address_carol])

    # It still points to bob
    assert_message_pass_through_relay(forwarder_address, [test_address_bob])

    assert_eventually(
      [%Relay{addr: ^forwarder_address, target_identifier: ^bob_id}] =
        StaticForwardingService.list_running_relays()
    )

    # If attribute is missing, it is not allowed to create any relay
    {:ok, channel_carol_2} = create_channel_with_credential(authority, carol, listener, %{})

    :ok =
      refute_register_relay(channel_carol_2, service_address, "anyalias", [test_address_carol])

    Ockam.Node.stop(forwarder_address)
    assert_eventually([] = StaticForwardingService.list_running_relays())
  end
end
