defmodule Ockam.Services.Authorization.Tests do
  @moduledoc false
  use ExUnit.Case, async: true

  alias Ockam.Identity

  alias Ockam.Message
  alias Ockam.Router

  alias Ockam.SecureChannel
  alias Ockam.SecureChannel.Crypto

  alias Ockam.Services.Echo

  require Logger

  setup_all do
    {:ok, listener_identity} = Identity.create()
    {:ok, listener_keypair} = Crypto.generate_dh_keypair()
    {:ok, attestation} = Identity.attest_purpose_key(listener_identity, listener_keypair)

    {:ok, channel_listener} =
      SecureChannel.create_listener(
        identity: listener_identity,
        encryption_options: [
          static_keypair: listener_keypair,
          static_key_attestation: attestation
        ]
      )

    on_exit(fn ->
      Ockam.Node.stop(channel_listener)
    end)

    [channel_listener: channel_listener]
  end

  test "Worker requiring local message", %{channel_listener: channel_listener} do
    {:ok, echoer} = Echo.create(authorization: [:is_local])

    {:ok, me} = Ockam.Node.register_random_address()

    Router.route(%Message{
      payload: "Hello local",
      onward_route: [echoer],
      return_route: [me]
    })

    assert_receive(%Message{onward_route: [^me], payload: "Hello local"}, 500)

    Router.route(%Message{
      payload: "Hello transport",
      onward_route: [echoer],
      return_route: [me],
      local_metadata: %{source: :channel, channel: :some_transport}
    })

    refute_receive(%Message{onward_route: [^me], payload: "Hello transport"}, 500)

    {:ok, channel} = create_channel([channel_listener])

    Router.route(%Message{
      payload: "Hello secure channel",
      onward_route: [channel, echoer],
      return_route: [me]
    })

    refute_receive(%Message{onward_route: [^me], payload: "Hello secure channel"}, 500)
  end

  test "Identity secure channel authorization", %{channel_listener: channel_listener} do
    {:ok, channel} = create_channel([channel_listener])

    {:ok, echoer} = Echo.create(authorization: [:from_secure_channel])

    {:ok, me} = Ockam.Node.register_random_address()
    Ockam.Router.route("VIA CHANNEL", [channel, echoer], [me])

    assert_receive(%Ockam.Message{onward_route: [^me], payload: "VIA CHANNEL"}, 500)

    Ockam.Router.route("WITHOUT CHANNEL", [echoer], [me])

    refute_receive(%Ockam.Message{onward_route: [^me], payload: "WITHOUT CHANNEL"}, 500)
  end

  test "Identity secure channel initiator authorization", %{channel_listener: channel_listener} do
    {:ok, channel} = create_channel([channel_listener], [:is_local])

    {:ok, me} = Ockam.Node.register_random_address()
    Ockam.Router.route("initiator from local", [me], [me])

    assert_receive(%Ockam.Message{onward_route: [^me], payload: "initiator from local"}, 500)

    Ockam.Router.route(%Ockam.Message{
      payload: "initiator from channel",
      onward_route: [channel, me],
      return_route: [me],
      local_metadata: %{source: :channel, channel: :some_transport}
    })

    refute_receive(%Ockam.Message{onward_route: [^me], payload: "initiator from channel"}, 500)
  end

  test "Identity secure channel responer authorization" do
    {:ok, listener_identity} = Identity.create()
    {:ok, listener_keypair} = Crypto.generate_dh_keypair()
    {:ok, attestation} = Identity.attest_purpose_key(listener_identity, listener_keypair)

    {:ok, listener} =
      SecureChannel.create_listener(
        identity: listener_identity,
        encryption_options: [
          static_keypair: listener_keypair,
          static_key_attestation: attestation
        ],
        responder_authorization: [:is_local]
      )

    {:ok, bob_channel} = create_channel([listener])

    {:ok, me} = Ockam.Node.register_random_address()
    Ockam.Router.route("VIA CHANNEL", [bob_channel, me], [me])

    receive do
      %Ockam.Message{
        onward_route: [^me],
        return_route: [responder | _]
      } ->
        Ockam.Router.route(%Ockam.Message{
          payload: "responder from channel",
          onward_route: [responder, me],
          return_route: [me],
          local_metadata: %{source: :channel, channel: :some_transport}
        })

        refute_receive(
          %Ockam.Message{onward_route: [^me], payload: "responder from channel"},
          500
        )

        Ockam.Router.route(%Ockam.Message{
          payload: "responder from local",
          onward_route: [responder, me],
          return_route: [me],
          local_metadata: %{source: :local}
        })

        assert_receive(%Ockam.Message{onward_route: [^me], payload: "responder from local"}, 500)
    after
      1000 ->
        raise "timeout receiving message via channel"
    end
  end

  test "forwarder authorization" do
    {:ok, service} =
      Ockam.Services.Forwarding.create(forwarder_options: [authorization: [:is_local]])

    {:ok, test_address} = Ockam.Node.register_random_address()

    register_message = %Message{
      onward_route: [service],
      payload: "",
      return_route: [test_address]
    }

    Router.route(register_message)

    assert_receive(
      %Message{
        onward_route: [^test_address],
        return_route: [forwarder_address]
      },
      5_000
    )

    local_message = %Message{
      onward_route: [forwarder_address, "smth"],
      payload: "hello",
      return_route: [test_address]
    }

    Router.route(local_message)

    assert_receive(%Message{payload: "hello", onward_route: [^test_address, "smth"]}, 500)

    channel_message = %Message{
      onward_route: [forwarder_address, "smth"],
      payload: "hello from channel",
      return_route: [test_address],
      local_metadata: %{source: :channel, channel: :tcp}
    }

    Router.route(channel_message)

    refute_receive(
      %Message{payload: "hello from channel", onward_route: [^test_address, "smth"]},
      500
    )
  end

  test "static forwarder authorization" do
    {:ok, service} =
      Ockam.Services.StaticForwarding.create(forwarder_options: [authorization: [:is_local]])

    {:ok, test_address} = Ockam.Node.register_random_address()

    register_message = %Message{
      onward_route: [service],
      payload: :bare.encode(test_address, :string),
      return_route: [test_address]
    }

    Router.route(register_message)

    assert_receive(
      %Message{
        onward_route: [^test_address],
        return_route: [forwarder_address]
      },
      5_000
    )

    local_message = %Message{
      onward_route: [forwarder_address, "smth"],
      payload: "hello",
      return_route: [test_address]
    }

    Router.route(local_message)

    assert_receive(%Message{payload: "hello", onward_route: [^test_address, "smth"]}, 500)

    channel_message = %Message{
      onward_route: [forwarder_address, "smth"],
      payload: "hello from channel",
      return_route: [test_address],
      local_metadata: %{source: :channel, channel: :tcp}
    }

    Router.route(channel_message)

    refute_receive(
      %Message{payload: "hello from channel", onward_route: [^test_address, "smth"]},
      500
    )
  end

  defp create_channel(route, authorization \\ []) do
    {:ok, identity} = Identity.create()

    {:ok, keypair} = Crypto.generate_dh_keypair()
    {:ok, attestation} = Identity.attest_purpose_key(identity, keypair)

    SecureChannel.create_channel(
      identity: identity,
      encryption_options: [static_keypair: keypair, static_key_attestation: attestation],
      route: route,
      authorization: authorization
    )
  end
end
