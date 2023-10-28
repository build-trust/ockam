defmodule Test.Services.StaticForwardingApiTest do
  use ExUnit.Case

  # Fail after 200ms of retrying with time between attempts 10ms
  use AssertEventually, timeout: 200, interval: 10

  alias Ockam.API.Client
  alias Ockam.Identity
  alias Ockam.SecureChannel

  alias Ockam.Services.Relay.StaticForwardingAPI
  alias Ockam.Services.Relay.Types.CreateRelayRequest
  alias Ockam.Services.Relay.Types.Relay

  test "crud" do
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

    {:ok, service_address} =
      StaticForwardingAPI.create(prefix: "forward_to", check_owner_on_delete: true)

    {:ok, service_address_no_enforcement} =
      StaticForwardingAPI.create(prefix: "forward_to", check_owner_on_delete: false)

    alias_str_1 = "test_static_forwarding_alias_1"
    alias_str_2 = "test_static_forwarding_alias_2"

    forwarder_address_1 = "forward_to_" <> alias_str_1
    forwarder_address_2 = "forward_to_" <> alias_str_2

    {:ok, resp} = Client.sync_request(:get, "/", nil, [bob_channel, service_address])
    assert %{status: 200, body: body} = resp
    assert {:ok, []} = Relay.decode_list_strict(body)

    req = %CreateRelayRequest{alias: alias_str_1, tags: %{"name" => "test_relay1"}}

    {:ok, resp} =
      Client.sync_request(:post, "/", CreateRelayRequest.encode!(req), [
        bob_channel,
        service_address
      ])

    assert %{status: 200, body: body} = resp

    assert {:ok, %Relay{target_identifier: ^bob_id, tags: %{"name" => "test_relay1"}}} =
             Relay.decode_strict(body)

    req = %CreateRelayRequest{alias: alias_str_2, tags: %{"name" => "test_relay2"}}

    {:ok, resp} =
      Client.sync_request(:post, "/", CreateRelayRequest.encode!(req), [
        bob_channel,
        service_address
      ])

    assert %{status: 200, body: body} = resp

    assert {:ok, %Relay{target_identifier: ^bob_id, tags: %{"name" => "test_relay2"}}} =
             Relay.decode_strict(body)

    {:ok, resp} = Client.sync_request(:get, "/", nil, [bob_channel, service_address])
    assert %{status: 200, body: body} = resp
    assert {:ok, [_, _]} = Relay.decode_list_strict(body)

    # Alice not allowed to overtake bob' relay
    req = %CreateRelayRequest{alias: alias_str_2, tags: %{"name" => "test_relay2"}}

    {:ok, resp} =
      Client.sync_request(:post, "/", CreateRelayRequest.encode!(req), [
        alice_channel,
        service_address
      ])

    assert %{status: 401} = resp

    assert {:ok, [%Relay{target_identifier: ^bob_id}, %Relay{target_identifier: ^bob_id}]} =
             Relay.decode_list_strict(body)

    # Alice not allowed to remove bob' relay
    {:ok, resp} =
      Client.sync_request(:delete, "/#{forwarder_address_1}", nil, [
        alice_channel,
        service_address
      ])

    assert %{status: 401} = resp

    # Bob can change its relay
    req = %CreateRelayRequest{alias: alias_str_2, tags: %{"name" => "changed!"}}

    {:ok, resp} =
      Client.sync_request(:post, "/", CreateRelayRequest.encode!(req), [
        bob_channel,
        service_address
      ])

    assert %{status: 200, body: body} = resp

    assert {:ok, %Relay{target_identifier: ^bob_id, tags: %{"name" => "changed!"}}} =
             Relay.decode_strict(body)

    # Bob can remove its own relay
    {:ok, resp} =
      Client.sync_request(:delete, "/#{forwarder_address_1}", nil, [bob_channel, service_address])

    assert %{status: 200} = resp

    {:ok, resp} = Client.sync_request(:get, "/", nil, [bob_channel, service_address])
    assert %{status: 200, body: body} = resp
    assert {:ok, [%Relay{addr: ^forwarder_address_2}]} = Relay.decode_list_strict(body)

    # Alice (and anyone) allowed to remove anyone relay if so configured..
    {:ok, resp} =
      Client.sync_request(:delete, "/#{forwarder_address_2}", nil, [
        alice_channel,
        service_address_no_enforcement
      ])

    assert %{status: 200} = resp

    {:ok, resp} = Client.sync_request(:get, "/", nil, [bob_channel, service_address])
    assert %{status: 200, body: body} = resp
    assert {:ok, []} = Relay.decode_list_strict(body)
  end
end
