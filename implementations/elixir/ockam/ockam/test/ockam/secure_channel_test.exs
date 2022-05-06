defmodule Ockam.SecureChannel.Tests.Wait do
  def until(f), do: f.() || until(f)
end

defmodule Ockam.SecureChannel.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.SecureChannel

  alias Ockam.Node
  alias Ockam.Router
  alias Ockam.SecureChannel
  alias Ockam.SecureChannel.Tests.Wait
  alias Ockam.Tests.Helpers.Echoer
  alias Ockam.Vault
  alias Ockam.Vault.Software, as: SoftwareVault

  setup do
    Node.register_address("test")
    on_exit(fn -> Node.unregister_address("test") end)
  end

  test "secure channel works" do
    {:ok, echoer} = Echoer.create([])

    {:ok, listener} = create_secure_channel_listener()
    {:ok, channel} = create_secure_channel([listener])

    message = %{
      payload: "hello",
      onward_route: [channel, echoer],
      return_route: ["test"]
    }

    Router.route(message)

    assert_receive %{
                     payload: "hello",
                     onward_route: ["test"],
                     return_route: [^channel, ^echoer]
                   },
                   1000
  end

  test "tunneled secure channel works" do
    {:ok, echoer} = Echoer.create([])

    {:ok, l1} = create_secure_channel_listener()
    {:ok, c1} = create_secure_channel([l1])

    {:ok, l2} = create_secure_channel_listener()
    {:ok, c2} = create_secure_channel([c1, l2])

    message = %{payload: "hello", onward_route: [c2, echoer], return_route: ["test"]}

    Router.route(message)
    assert_receive %{payload: "hello", onward_route: ["test"], return_route: [^c2, ^echoer]}, 1000
  end

  test "double-tunneled secure channel works" do
    {:ok, echoer} = Echoer.create([])

    {:ok, l1} = create_secure_channel_listener()
    {:ok, c1} = create_secure_channel([l1])

    {:ok, l2} = create_secure_channel_listener()
    {:ok, c2} = create_secure_channel([c1, l2])

    {:ok, l3} = create_secure_channel_listener()
    {:ok, c3} = create_secure_channel([c2, l3])

    message = %{payload: "hello", onward_route: [c3, echoer], return_route: ["test"]}

    Router.route(message)

    assert_receive %{payload: "hello", onward_route: ["test"], return_route: [^c3, ^echoer]},
                   10_000
  end

  test "many times tunneled secure channel works" do
    {:ok, echoer} = Echoer.create([])

    # pick a random number between 4 and 10, create that many tunnels
    {:ok, tunneled} =
      1..Enum.random(4..10)
      |> Enum.map(fn i ->
        {:ok, listener} = create_secure_channel_listener()
        {i, listener}
      end)
      |> Enum.reduce(nil, fn
        {_i, listener}, nil -> create_secure_channel([listener])
        {_i, listener}, {:ok, previous} -> create_secure_channel([previous, listener])
      end)

    message = %{
      payload: "hello",
      onward_route: [tunneled, echoer],
      return_route: ["test"]
    }

    Router.route(message)

    assert_receive %{
                     payload: "hello",
                     onward_route: ["test"],
                     return_route: [^tunneled, ^echoer]
                   },
                   10_000
  end

  defp create_secure_channel_listener() do
    {:ok, vault} = SoftwareVault.init()
    {:ok, identity} = Vault.secret_generate(vault, type: :curve25519)
    SecureChannel.create_listener(vault: vault, identity_keypair: identity)
  end

  defp create_secure_channel(route_to_listener) do
    {:ok, vault} = SoftwareVault.init()
    {:ok, identity} = Vault.secret_generate(vault, type: :curve25519)

    {:ok, c} =
      SecureChannel.create(route: route_to_listener, vault: vault, identity_keypair: identity)

    Wait.until(fn -> SecureChannel.established?(c) end)
    {:ok, c}
  end
end
