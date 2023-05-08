defmodule Ockam.Services.Authorization.Tests do
  @moduledoc false
  use ExUnit.Case, async: true

  alias Ockam.Identity

  alias Ockam.Message
  alias Ockam.Router

  alias Ockam.Services.Echo

  alias Ockam.Vault.Software, as: SoftwareVault

  require Logger

  setup_all do
    {:ok, vault} = SoftwareVault.init()
    {:ok, keypair} = Ockam.Vault.secret_generate(vault, type: :curve25519)

    {:ok, channel_listener} =
      Ockam.SecureChannel.create_listener(vault: vault, static_keypair: keypair)

    on_exit(fn ->
      Ockam.Node.stop(channel_listener)
    end)

    [vault: vault, channel_listener: channel_listener]
  end

  test "Worker requiring secure channel", %{vault: vault, channel_listener: channel_listener} do
    {:ok, echoer} = Echo.create(authorization: [:from_secure_channel])

    {:ok, me} = Ockam.Node.register_random_address()

    Router.route(%Message{
      payload: "Hello fake secure channel",
      onward_route: [echoer],
      return_route: [me],
      local_metadata: %{source: :channel, channel: :secure_channel}
    })

    assert_receive(%Message{onward_route: [^me], payload: "Hello fake secure channel"}, 500)

    Router.route(%Message{
      payload: "Hello local",
      onward_route: [echoer],
      return_route: [me]
    })

    refute_receive(%Message{onward_route: [^me], payload: "Hello local"}, 500)

    {:ok, keypair} = Ockam.Vault.secret_generate(vault, type: :curve25519)

    {:ok, channel} =
      Ockam.SecureChannel.create(vault: vault, route: [channel_listener], static_keypair: keypair)

    Router.route(%Message{
      payload: "Hello secure channel",
      onward_route: [channel, echoer],
      return_route: [me]
    })

    assert_receive(%Message{onward_route: [^me], payload: "Hello secure channel"}, 500)
  end

  test "Worker requiring local message", %{vault: vault, channel_listener: channel_listener} do
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

    {:ok, keypair} = Ockam.Vault.secret_generate(vault, type: :curve25519)

    {:ok, channel} =
      Ockam.SecureChannel.create(vault: vault, route: [channel_listener], static_keypair: keypair)

    Router.route(%Message{
      payload: "Hello secure channel",
      onward_route: [channel, echoer],
      return_route: [me]
    })

    refute_receive(%Message{onward_route: [^me], payload: "Hello secure channel"}, 500)
  end

  test "Identity secure channel authorization" do
    {:ok, vault} = SoftwareVault.init()
    {:ok, listener_identity, _id} = Identity.create(Ockam.Identity.Stub)

    {:ok, listener} =
      Ockam.Identity.SecureChannel.create_listener(
        identity: listener_identity,
        encryption_options: [vault: vault]
      )

    {:ok, bob, _bob_id} = Identity.create(Ockam.Identity.Stub)

    {:ok, bob_channel} =
      Ockam.Identity.SecureChannel.create_channel(
        identity: bob,
        encryption_options: [vault: vault],
        route: [listener]
      )

    {:ok, echoer} = Echo.create(authorization: [:from_identiy_secure_channel])

    {:ok, me} = Ockam.Node.register_random_address()
    Ockam.Router.route("VIA CHANNEL", [bob_channel, echoer], [me])

    assert_receive(%Ockam.Message{onward_route: [^me], payload: "VIA CHANNEL"}, 500)

    Ockam.Router.route("WITHOUT CHANNEL", [echoer], [me])

    refute_receive(%Ockam.Message{onward_route: [^me], payload: "WITHOUT CHANNEL"}, 500)
  end

  test "Identity secure channel initiator authorization" do
    {:ok, vault} = SoftwareVault.init()
    {:ok, listener_identity, _id} = Identity.create(Ockam.Identity.Stub)

    {:ok, listener} =
      Ockam.Identity.SecureChannel.create_listener(
        identity: listener_identity,
        encryption_options: [vault: vault]
      )

    {:ok, bob, _bob_id} = Identity.create(Ockam.Identity.Stub)

    {:ok, bob_channel} =
      Ockam.Identity.SecureChannel.create_channel(
        identity: bob,
        encryption_options: [vault: vault],
        route: [listener],
        authorization: [:is_local]
      )

    {:ok, me} = Ockam.Node.register_random_address()
    Ockam.Router.route("initiator from local", [bob_channel, me], [me])

    assert_receive(%Ockam.Message{onward_route: [^me], payload: "initiator from local"}, 500)

    Ockam.Router.route(%Ockam.Message{
      payload: "initiator from channel",
      onward_route: [bob_channel, me],
      return_route: [me],
      local_metadata: %{source: :channel, channel: :some_transport}
    })

    refute_receive(%Ockam.Message{onward_route: [^me], payload: "initiator from channel"}, 500)
  end

  test "Identity secure channel responer authorization" do
    {:ok, vault} = SoftwareVault.init()
    {:ok, listener_identity, _id} = Identity.create(Ockam.Identity.Stub)

    {:ok, listener} =
      Ockam.Identity.SecureChannel.create_listener(
        identity: listener_identity,
        encryption_options: [vault: vault],
        responder_authorization: [:is_local]
      )

    {:ok, bob, _bob_id} = Identity.create(Ockam.Identity.Stub)

    {:ok, bob_channel} =
      Ockam.Identity.SecureChannel.create_channel(
        identity: bob,
        encryption_options: [vault: vault],
        route: [listener]
      )

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
end
