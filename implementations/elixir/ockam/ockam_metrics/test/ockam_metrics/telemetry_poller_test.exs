defmodule Ockam.Metrics.TelemetryPoller.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Metrics.TelemetryPoller

  alias Ockam.Identity
  alias Ockam.Metrics.TelemetryPoller
  alias Ockam.SecureChannel

  setup do
    {:ok, me} = Ockam.Node.register_random_address()
    on_exit(fn -> Ockam.Node.unregister_address("test") end)
    {:ok, me: me}
  end

  test "secure channels metrics", %{me: self_addr} do
    {:ok, alice} = Identity.create()
    alice_id = Identity.get_identifier(alice)
    {:ok, alice_keypair} = SecureChannel.Crypto.generate_dh_keypair()
    {:ok, alice_attestation} = Identity.attest_purpose_key(alice, alice_keypair)

    {:ok, listener} =
      SecureChannel.create_listener(
        identity: alice,
        encryption_options: [
          static_keypair: alice_keypair,
          static_key_attestation: alice_attestation
        ]
      )

    {:ok, bob} = Identity.create()
    {:ok, bob_keypair} = SecureChannel.Crypto.generate_dh_keypair()
    {:ok, bob_attestation} = Identity.attest_purpose_key(bob, bob_keypair)

    {:ok, channel} =
      SecureChannel.create_channel(
        identity: bob,
        encryption_options: [static_keypair: bob_keypair, static_key_attestation: bob_attestation],
        route: [listener]
      )

    channel_pid = Ockam.Node.whereis(channel)
    ref1 = Process.monitor(channel_pid)

    assert {:ok, alice} == SecureChannel.get_remote_identity(channel)
    assert {:ok, alice_id} == SecureChannel.get_remote_identity_id(channel)

    {:ok, channel2} =
      SecureChannel.create_channel(
        identity: bob,
        encryption_options: [static_keypair: bob_keypair, static_key_attestation: bob_attestation],
        route: [listener]
      )

    Ockam.Node.register_address("echo")

    # Make sure both channels are fully established before checking metrics.
    # it's because initiator' create_channel returns after sending it's own identity to responder,
    # we don't know when that is processed by responder.  Rather than than adding sleep() here on tests
    Ockam.Router.route(%{
      payload: "hello1",
      onward_route: [channel, "echo"],
      return_route: [self_addr]
    })

    Ockam.Router.route(%{
      payload: "hello2",
      onward_route: [channel2, "echo"],
      return_route: [self_addr]
    })

    # This is the echo service
    receive do
      %Ockam.Message{onward_route: ["echo"], return_route: return_route} = msg ->
        Ockam.Router.route(%Ockam.Message{
          msg
          | return_route: ["echo"],
            onward_route: return_route
        })
    end

    receive do
      %Ockam.Message{onward_route: ["echo"], return_route: return_route} = msg ->
        Ockam.Router.route(%Ockam.Message{
          msg
          | return_route: ["echo"],
            onward_route: return_route
        })
    end

    # This is what we get back from echo service
    assert_receive %Ockam.Message{payload: "hello1", return_route: return_route}
    assert_receive %Ockam.Message{payload: "hello2"}

    [receiver_addr, _] = return_route
    receiver_pid = Ockam.Node.whereis(receiver_addr)
    ref2 = Process.monitor(receiver_pid)

    %{
      handshake_initiators: [],
      handshake_responders: [],
      data_initiators: [_, _],
      data_responders: [_, _]
    } = TelemetryPoller.secure_channels()

    SecureChannel.disconnect(channel)

    # Be sure to wait until both initiator and responder have really stopped
    assert_receive {:DOWN, ^ref1, _, _, _}
    assert_receive {:DOWN, ^ref2, _, _, _}

    %{
      handshake_initiators: [],
      handshake_responders: [],
      data_initiators: [_],
      data_responders: [_]
    } = TelemetryPoller.secure_channels()
  end
end
