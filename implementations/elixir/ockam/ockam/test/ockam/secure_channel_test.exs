defmodule Ockam.SecureChannel.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.SecureChannel

  alias Ockam.Message
  alias Ockam.Node
  alias Ockam.Router
  alias Ockam.SecureChannel
  alias Ockam.Tests.Helpers.Echoer
  alias Ockam.Vault
  alias Ockam.Vault.Software, as: SoftwareVault

  setup do
    Node.register_address("test")
    on_exit(fn -> Node.unregister_address("test") end)
  end

  defp man_in_the_middle(callback) do
    receive do
      message ->
        initiator = Message.return_route(message)
        message |> Message.forward_trace() |> Router.route()
        man_in_the_middle(callback, initiator, 1)
    end
  end

  defp man_in_the_middle(callback, initiator, n) do
    receive do
      %Message{return_route: ^initiator} = message when n >= 3 ->
        callback.(message, n - 3) |> Enum.each(&Router.route/1)

      %Message{} = message ->
        message |> Message.forward_trace() |> Router.route()
    end

    man_in_the_middle(callback, initiator, n + 1)
  end

  test "secure channel drop packets" do
    # Drop even messages from initiator -> target (after channel established)
    # if msgs  a,b,c,d,e,f  are send,  msgs b,d,f are delivered to the secure other
    # end secure channel.
    drop_evens = fn message, n ->
      if rem(n, 2) == 0 do
        []
      else
        [Message.forward_trace(message)]
      end
    end

    {:ok, _} =
      Task.start_link(fn ->
        Node.register_address("man_in_the_middle")
        man_in_the_middle(drop_evens)
      end)

    {:ok, listener} = create_secure_channel_listener()
    {:ok, channel} = create_secure_channel(["man_in_the_middle", listener])

    # Send 50 messages, only the odd ones are received and decrypted ok, the others
    # are lost
    0..50
    |> Enum.each(fn i ->
      message = %{
        payload: :erlang.term_to_binary(i),
        onward_route: [channel, "test"],
        return_route: []
      }

      Router.route(message)

      if rem(i, 2) == 1 do
        receive do
          %Message{payload: payload} ->
            assert i == :erlang.binary_to_term(payload)
        after
          1000 ->
            flunk("Message #{i} didn't arrive")
        end
      end
    end)

    refute_receive(_, 100)
  end

  test "secure channel replay attack" do
    replay = fn message, _n ->
      m = Message.forward_trace(message)
      [m, m]
    end

    {:ok, _} =
      Task.start_link(fn ->
        Node.register_address("man_in_the_middle")
        man_in_the_middle(replay)
      end)

    {:ok, listener} = create_secure_channel_listener()
    {:ok, channel} = create_secure_channel(["man_in_the_middle", listener])

    # Send 50 messages, all received.  Duplicates ones are discarded and don't
    # affect the decryptor' state.
    0..50
    |> Enum.each(fn i ->
      message = %{
        payload: :erlang.term_to_binary(i),
        onward_route: [channel, "test"],
        return_route: []
      }

      Router.route(message)

      receive do
        %Message{payload: payload} ->
          assert i == :erlang.binary_to_term(payload)
      after
        1000 ->
          flunk("Message #{i} didn't arrive")
      end
    end)

    refute_receive(_, 100)
  end

  test "secure channel trash packets" do
    replay = fn %Message{payload: payload} = message, n ->
      if rem(n, 2) == 0 do
        # Payload is actually _not_ the raw encrypted bytes..  it's the encrypted bytes encoded with bare.
        # That means that we can have two different kind of "bad" packets:  things that can't
        # be decoded from bare,  and things that can be decoded from bare, but then can't be decrypted.
        # We put both here.
        trash1 = %Message{message | payload: payload <> "s"} |> Message.forward_trace()
        {:ok, raw, ""} = :bare.decode(payload, :data)

        trash2 =
          trash =
          %Message{message | payload: :bare.encode(raw <> "s", :data)} |> Message.forward_trace()

        [trash1, trash2]
      else
        [Message.forward_trace(message)]
      end
    end

    {:ok, _} =
      Task.start_link(fn ->
        Node.register_address("man_in_the_middle")
        man_in_the_middle(replay)
      end)

    {:ok, listener} = create_secure_channel_listener()
    {:ok, channel} = create_secure_channel(["man_in_the_middle", listener])

    # Send 50 messages, only the odd ones are received and decrypted ok, the others
    # are dropped because they were modified on the fly, so failed to decrypt
    0..50
    |> Enum.each(fn i ->
      message = %{
        payload: :erlang.term_to_binary(i),
        onward_route: [channel, "test"],
        return_route: []
      }

      Router.route(message)

      if rem(i, 2) == 1 do
        receive do
          %Message{payload: payload} ->
            assert i == :erlang.binary_to_term(payload)
        after
          1000 ->
            flunk("Message #{i} didn't arrive")
        end
      end
    end)

    refute_receive(_, 100)
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

    {:ok, c}
  end
end
