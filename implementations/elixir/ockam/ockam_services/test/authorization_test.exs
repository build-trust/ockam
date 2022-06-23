defmodule Ockam.Services.Authorization.Tests do
  @moduledoc false
  use ExUnit.Case, async: true

  alias Ockam.Identity

  alias Ockam.Message
  alias Ockam.Router

  alias Ockam.Services.AuthorizationConfig
  alias Ockam.Services.Echo

  alias Ockam.Vault.Software, as: SoftwareVault

  require Logger

  setup_all do
    {:ok, vault} = SoftwareVault.init()
    {:ok, identity} = Ockam.Vault.secret_generate(vault, type: :curve25519)

    {:ok, channel_listener} =
      Ockam.SecureChannel.create_listener(vault: vault, identity_keypair: identity)

    on_exit(fn ->
      Ockam.Node.stop(channel_listener)
    end)

    [vault: vault, channel_listener: channel_listener]
  end

  test "Worker requiring secure channel", %{vault: vault, channel_listener: channel_listener} do
    {:ok, echoer} = Echo.create(authorization: AuthorizationConfig.secure_channel())

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

    {:ok, kp} = Ockam.Vault.secret_generate(vault, type: :curve25519)

    {:ok, channel} =
      Ockam.SecureChannel.create(vault: vault, route: [channel_listener], identity_keypair: kp)

    Router.route(%Message{
      payload: "Hello secure channel",
      onward_route: [channel, echoer],
      return_route: [me]
    })

    assert_receive(%Message{onward_route: [^me], payload: "Hello secure channel"}, 500)
  end

  test "Worker requiring local message", %{vault: vault, channel_listener: channel_listener} do
    {:ok, echoer} = Echo.create(authorization: AuthorizationConfig.local())

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

    {:ok, kp} = Ockam.Vault.secret_generate(vault, type: :curve25519)

    {:ok, channel} =
      Ockam.SecureChannel.create(vault: vault, route: [channel_listener], identity_keypair: kp)

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

    {:ok, echoer} = Echo.create(authorization: AuthorizationConfig.identity_secure_channel())

    {:ok, me} = Ockam.Node.register_random_address()
    Ockam.Router.route("VIA CHANNEL", [bob_channel, echoer], [me])

    assert_receive(%Ockam.Message{onward_route: [^me], payload: "VIA CHANNEL"}, 500)

    Ockam.Router.route("WITHOUT CHANNEL", [echoer], [me])

    refute_receive(%Ockam.Message{onward_route: [^me], payload: "WITHOUT CHANNEL"}, 500)
  end
end
