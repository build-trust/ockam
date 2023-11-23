defmodule Test.Services.StaticForwardingApiTest do
  use ExUnit.Case

  # Fail after 200ms of retrying with time between attempts 10ms
  use AssertEventually, timeout: 200, interval: 10

  alias Ockam.Credential.AttributeStorageETS, as: AttributeStorage

  alias Ockam.API.Client
  alias Ockam.Identity
  alias Ockam.SecureChannel

  alias Ockam.Services.Relay.StaticForwardingAPI
  alias Ockam.Services.Relay.Types.CreateRelayRequest
  alias Ockam.Services.Relay.Types.Relay

  test "crud" do
    :ok = AttributeStorage.init()
    {:ok, authority_identity} = Identity.create()
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
        authorities: [authority_identity]
      )

    {:ok, bob} = Identity.create()
    {:ok, alice} = Identity.create()
    {:ok, carol} = Identity.create()
    bob_id = Identity.get_identifier(bob)
    alice_id = Identity.get_identifier(alice)
    {:ok, bob_keypair} = SecureChannel.Crypto.generate_dh_keypair()
    {:ok, bob_attestation} = Identity.attest_purpose_key(bob, bob_keypair)
    {:ok, alice_keypair} = SecureChannel.Crypto.generate_dh_keypair()
    {:ok, alice_attestation} = Identity.attest_purpose_key(alice, alice_keypair)
    {:ok, carol_keypair} = SecureChannel.Crypto.generate_dh_keypair()
    {:ok, carol_attestation} = Identity.attest_purpose_key(carol, carol_keypair)

    alias_str_1 = "test_static_forwarding_alias_1"
    alias_str_2 = "test_static_forwarding_alias_2"

    {:ok, bob_credential} =
      Identity.issue_credential(authority_identity, bob_id, %{"allow_relay_address" => "*"}, 100)

    {:ok, alice_credential} =
      Identity.issue_credential(
        authority_identity,
        alice_id,
        %{"allow_relay_address" => alias_str_1},
        100
      )

    {:ok, bob_channel} =
      SecureChannel.create_channel(
        identity: bob,
        encryption_options: [static_keypair: bob_keypair, static_key_attestation: bob_attestation],
        authorities: [authority_identity],
        credentials: [bob_credential],
        route: [listener]
      )

    {:ok, alice_channel} =
      SecureChannel.create_channel(
        identity: alice,
        encryption_options: [
          static_keypair: alice_keypair,
          static_key_attestation: alice_attestation
        ],
        authorities: [authority_identity],
        credentials: [alice_credential],
        route: [listener]
      )

    {:ok, carol_channel} =
      SecureChannel.create_channel(
        identity: carol,
        encryption_options: [
          static_keypair: carol_keypair,
          static_key_attestation: carol_attestation
        ],
        authorities: [authority_identity],
        credentials: [],
        route: [listener]
      )

    {:ok, service_address} =
      StaticForwardingAPI.create(prefix: "forward_to", check_owner_on_delete: true)

    forwarder_address_1 = "forward_to_" <> alias_str_1
    forwarder_address_2 = "forward_to_" <> alias_str_2

    on_exit(fn ->
      Ockam.Node.stop(forwarder_address_1)
      Ockam.Node.stop(forwarder_address_2)
    end)

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

    # Alice allowed to overtake bob' relay at alias_str_1
    req = %CreateRelayRequest{alias: alias_str_1, tags: %{"name" => "test_relay1_alice"}}

    {:ok, resp} =
      Client.sync_request(:post, "/", CreateRelayRequest.encode!(req), [
        alice_channel,
        service_address
      ])

    assert %{status: 200, body: body} = resp

    assert {:ok, %Relay{target_identifier: ^alice_id, tags: %{"name" => "test_relay1_alice"}}} =
             Relay.decode_strict(body)

    # Carol not allowed to take any relay
    req = %CreateRelayRequest{alias: alias_str_1, tags: %{"name" => "carol"}}

    {:ok, resp} =
      Client.sync_request(:post, "/", CreateRelayRequest.encode!(req), [
        carol_channel,
        service_address
      ])

    assert %{status: 401} = resp

    req = %CreateRelayRequest{alias: "anyalias", tags: %{"name" => "carol"}}

    {:ok, resp} =
      Client.sync_request(:post, "/", CreateRelayRequest.encode!(req), [
        carol_channel,
        service_address
      ])

    assert %{status: 401} = resp

    # Carol not allowed to remove relay
    {:ok, resp} =
      Client.sync_request(:delete, "/#{forwarder_address_1}", nil, [
        carol_channel,
        service_address
      ])

    assert %{status: 401} = resp

    # Bob can remove it
    {:ok, resp} =
      Client.sync_request(:delete, "/#{forwarder_address_1}", nil, [
        bob_channel,
        service_address
      ])

    assert %{status: 200} = resp

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
      Client.sync_request(:delete, "/#{alias_str_1}", nil, [bob_channel, service_address])

    assert %{status: 200} = resp

    assert_eventually(
      (
        {:ok, resp} = Client.sync_request(:get, "/", nil, [bob_channel, service_address])
        %{status: 200, body: body} = resp
        {:ok, [%Relay{addr: ^forwarder_address_2}]} = Relay.decode_list_strict(body)
      )
    )
  end
end
