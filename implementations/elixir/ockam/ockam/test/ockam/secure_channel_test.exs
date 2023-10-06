defmodule Ockam.SecureChannel.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.SecureChannel

  alias Ockam.Credential.AttributeStorageETS, as: AttributeStorage
  alias Ockam.Identity
  alias Ockam.Message
  alias Ockam.Node
  alias Ockam.Router
  alias Ockam.SecureChannel
  alias Ockam.SecureChannel.Crypto
  alias Ockam.Tests.Helpers.Echoer

  @identity_impl Ockam.Identity.Stub

  setup do
    Node.register_address("test")
    {:ok, alice} = Identity.create()
    {:ok, bob} = Identity.create()
    on_exit(fn -> Node.unregister_address("test") end)

    # TODO: rework the relationship on credential exchange API, attribute storage and secure channel
    :ok = AttributeStorage.init()
    {:ok, alice: alice, bob: bob}
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
      %Message{return_route: ^initiator} = message when n > 2 ->
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

  test "local secure channel", %{alice: alice, bob: bob} do
    {:ok, listener} = create_secure_channel_listener(alice)

    {:ok, channel} = create_secure_channel([listener], bob)

    channel_pid = Ockam.Node.whereis(channel)

    ref1 = Process.monitor(channel_pid)

    assert {:ok, alice} == SecureChannel.get_remote_identity(channel)
    assert {:ok, Identity.get_identifier(alice)} == SecureChannel.get_remote_identity_id(channel)

    assert {:ok, alice, Identity.get_identifier(alice)} ==
             SecureChannel.get_remote_identity_with_id(channel)

    {:ok, me} = Ockam.Node.register_random_address()
    Router.route("PING!", [channel, me], [me])

    assert_receive %Ockam.Message{
      onward_route: [^me],
      payload: "PING!",
      return_route: return_route,
      local_metadata: %{identity_id: id, identity: _identity, channel: :secure_channel}
    }

    assert id == Identity.get_identifier(bob)

    # Hacky way to get the receiver' pid, so we can monitor it and ensure it get terminated
    # after disconnection
    [receiver_addr, _] = return_route
    receiver_pid = Ockam.Node.whereis(receiver_addr)
    ref2 = Process.monitor(receiver_pid)

    Router.route("PONG!", return_route, [me])

    assert_receive %Ockam.Message{
      onward_route: [^me],
      payload: "PONG!",
      return_route: [^channel | _],
      local_metadata: %{identity_id: id, identity: _identity, channel: :secure_channel}
    }

    assert id == Identity.get_identifier(alice)

    SecureChannel.disconnect(channel)
    assert_receive {:DOWN, ^ref1, _, _, _}
    assert_receive {:DOWN, ^ref2, _, _, _}
  end

  test "identity channel inner address is protected", %{alice: alice, bob: bob} do
    ## Inner address is the one pointing to the other peer.
    ## This just test that it don't pass messages around, as
    ## the message will fail to be decrypted

    {:ok, listener} = create_secure_channel_listener(alice)

    {:ok, channel} = create_secure_channel([listener], bob)

    {:ok, bob_inner_address} = Ockam.AsymmetricWorker.get_inner_address(channel)

    {:ok, me} = Ockam.Node.register_random_address()

    Router.route("PING!", [bob_inner_address, me], [me])

    refute_receive %Ockam.Message{
      onward_route: [^me],
      payload: "PING!"
    }
  end

  test "additional metadata", %{alice: alice, bob: bob} do
    {:ok, listener} = create_secure_channel_listener(alice, %{foo: :bar})
    {:ok, channel} = create_secure_channel([listener], bob, %{bar: :foo})

    {:ok, me} = Ockam.Node.register_random_address()
    Router.route("PING!", [channel, me], [me])

    assert_receive %Ockam.Message{
      onward_route: [^me],
      payload: "PING!",
      return_route: return_route,
      local_metadata: %{
        identity_id: _id,
        identity: _identity,
        channel: :secure_channel,
        foo: :bar
      }
    }

    Router.route("PONG!", return_route, [me])

    assert_receive %Ockam.Message{
      onward_route: [^me],
      payload: "PONG!",
      return_route: [^channel | _],
      local_metadata: %{
        identity_id: _id,
        identity: _identity,
        channel: :secure_channel,
        bar: :foo
      }
    }
  end

  test "initiator trust policy", %{alice: alice, bob: bob} do
    {:ok, listener} = create_secure_channel_listener(alice, %{foo: :bar})

    {:ok, keypair} = Crypto.generate_dh_keypair()
    attestation = Identity.attest_purpose_key(bob, keypair)

    {:error, _reason} =
      SecureChannel.create_channel(
        [
          identity: bob,
          route: [listener],
          encryption_options: [static_keypair: keypair, static_key_attestation: attestation],
          additional_metadata: %{bar: :foo},
          trust_policies: [fn _me, _contact -> {:error, :test} end]
        ],
        2000
      )
  end

  test "responder trust policy", %{alice: alice, bob: bob} do
    {:ok, keypair} = Crypto.generate_dh_keypair()
    {:ok, attestation} = Identity.attest_purpose_key(alice, keypair)

    {:ok, listener} =
      SecureChannel.create_listener(
        identity: alice,
        encryption_options: [static_keypair: keypair, static_key_attestation: attestation],
        additional_metadata: %{foo: :bar},
        trust_policies: [fn _me, _contact -> {:error, :test} end]
      )

    {:ok, channel} = create_secure_channel([listener], bob, %{bar: :foo})

    {:ok, me} = Ockam.Node.register_random_address()
    Router.route("PING!", [channel, me], [me])

    refute_receive %Ockam.Message{
      onward_route: [^me],
      payload: "PING!"
    }
  end

  test "credential in handshake accepted", %{
    alice: alice,
    bob: bob
  } do
    {:ok, authority} = Identity.create()

    alice_attributes = %{"role" => "server"}
    alice_id = Identity.get_identifier(alice)
    bob_id = Identity.get_identifier(bob)
    {:ok, keypair} = Crypto.generate_dh_keypair()
    {:ok, attestation} = Identity.attest_purpose_key(alice, keypair)

    {:ok, alice_credential} =
      Identity.issue_credential(authority, alice_id, alice_attributes, 100)

    {:ok, listener} =
      SecureChannel.create_listener(
        identity: alice,
        encryption_options: [static_keypair: keypair, static_key_attestation: attestation],
        authorities: [authority],
        credentials: [alice_credential]
      )

    bob_attributes = %{"role" => "member"}
    {:ok, keypair} = Crypto.generate_dh_keypair()
    {:ok, attestation} = Identity.attest_purpose_key(bob, keypair)
    {:ok, bob_credential} = Identity.issue_credential(authority, bob_id, bob_attributes, 100)

    {:ok, channel} =
      SecureChannel.create_channel(
        [
          identity: bob,
          encryption_options: [static_keypair: keypair, static_key_attestation: attestation],
          route: [listener],
          authorities: [authority],
          credentials: [bob_credential]
        ],
        3000
      )

    {:ok, me} = Ockam.Node.register_random_address()

    Router.route("PING!", [channel, me], [me])

    # This to make sure receiver end has fully completed the handshake, and so processes our
    # credentials.
    assert_receive %Ockam.Message{
      onward_route: [^me],
      payload: "PING!",
      return_route: [_channel, ^me],
      local_metadata: %{identity_id: ^bob_id, channel: :secure_channel}
    }

    # Check that attributes had been stored
    assert bob_attributes == AttributeStorage.get_attributes(bob_id)
    # The client itself also store server' credential presented
    assert alice_attributes == AttributeStorage.get_attributes(alice_id)

    # Secure channel is terminated if we present invalid credential

    # Credential by unknown authority
    {:ok, wrong_authority} = Identity.create()
    attributes = %{"role" => "attacker"}

    {:ok, wrong_credential} = Identity.issue_credential(wrong_authority, bob_id, attributes, 100)

    # {:ok, channel} =
    {:error, _} =
      SecureChannel.create_channel(
        [
          identity: bob,
          encryption_options: [static_keypair: keypair, static_key_attestation: attestation],
          route: [listener],
          credentials: [wrong_credential],
          authority: [authority]
        ],
        1000
      )

    # Router.route("PING!", [channel, me], [me])

    # refute_receive %Ockam.Message{
    #  onward_route: [^me],
    #  payload: "PING!",
    #  return_route: [_channel, ^me],
    #  local_metadata: %{identity_id: ^bob_id, channel: :secure_channel}
    # }

    # Credential for another identifier
    attributes = %{"role" => "attacker"}
    {:ok, wrong_credential} = Identity.issue_credential(authority, alice_id, attributes, 100)

    {:ok, channel} =
      SecureChannel.create_channel(
        [
          identity: bob,
          encryption_options: [static_keypair: keypair, static_key_attestation: attestation],
          route: [listener],
          credentials: [wrong_credential],
          authorities: [authority]
        ],
        1000
      )

    Router.route("PING!", [channel, me], [me])

    refute_receive %Ockam.Message{
      onward_route: [^me],
      payload: "PING!",
      return_route: [_channel, ^me],
      local_metadata: %{identity_id: ^bob_id, channel: :secure_channel}
    }

    # Credential by wrong authority on server side
    {:error, _} =
      SecureChannel.create_channel(
        [
          identity: bob,
          encryption_options: [static_keypair: keypair, static_key_attestation: attestation],
          route: [listener],
          authorities: [wrong_authority]
        ],
        1000
      )
  end

  defp create_secure_channel_listener() do
    {:ok, identity} = Identity.create()
    create_secure_channel_listener(identity)
  end

  defp create_secure_channel_listener(identity) do
    create_secure_channel_listener(identity, %{})
  end

  defp create_secure_channel_listener(identity, additional_metadata) do
    {:ok, keypair} = Crypto.generate_dh_keypair()
    {:ok, attestation} = Identity.attest_purpose_key(identity, keypair)

    SecureChannel.create_listener(
      identity: identity,
      encryption_options: [static_keypair: keypair, static_key_attestation: attestation],
      additional_metadata: additional_metadata
    )
  end

  defp create_secure_channel(route_to_listener) do
    {:ok, identity} = Identity.create()
    create_secure_channel(route_to_listener, identity)
  end

  defp create_secure_channel(route_to_listener, identity) do
    create_secure_channel(route_to_listener, identity, %{})
  end

  defp create_secure_channel(route_to_listener, identity, additional_metadata) do
    {:ok, keypair} = Crypto.generate_dh_keypair()
    {:ok, attestation} = Identity.attest_purpose_key(identity, keypair)

    {:ok, c} =
      SecureChannel.create_channel(
        identity: identity,
        route: route_to_listener,
        encryption_options: [static_keypair: keypair, static_key_attestation: attestation],
        additional_metadata: additional_metadata
      )

    {:ok, c}
  end
end
