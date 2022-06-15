defmodule Ockam.Services.Authorization.Tests do
  @moduledoc false
  use ExUnit.Case, async: true

  alias Ockam.Message
  alias Ockam.Router

  alias Ockam.Services.AuthorizationConfig
  alias Ockam.Services.Echo

  require Logger

  setup_all do
    {:ok, vault} = Ockam.Vault.Software.init()
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

    wait_for_channel(channel)

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

    wait_for_channel(channel)

    Router.route(%Message{
      payload: "Hello secure channel",
      onward_route: [channel, echoer],
      return_route: [me]
    })

    refute_receive(%Message{onward_route: [^me], payload: "Hello secure channel"}, 500)
  end

  def wait_for_channel(channel) do
    case Ockam.SecureChannel.established?(channel) do
      true ->
        :ok

      false ->
        :timer.sleep(100)
        wait_for_channel(channel)
    end
  end
end
