defmodule Ockam.Channel.Handshake.Tests do
  use ExUnit.Case, async: true

  alias Ockam.Channel
  alias Ockam.Channel.Protocol
  alias Ockam.Channel.Handshake
  alias Ockam.Vault.KeyPair

  alias Ockam.Test.Fixtures

  setup context do
    if context[:vectors] == true do
      {:ok, [vectors: Fixtures.vectors()]}
    else
      {:ok, []}
    end
  end

  test "handshake without transport" do
    s = KeyPair.new(:x25519)
    e = KeyPair.new(:x25519)
    rs = KeyPair.new(:x25519)
    re = KeyPair.new(:x25519)
    initiator_opts = %{protocol: "Noise_XX_25519_AESGCM_SHA256", s: s, e: e, rs: rs, re: re}
    responder_opts = %{protocol: "Noise_XX_25519_AESGCM_SHA256", s: rs, e: re, rs: s, re: e}

    assert {:ok, init} = Channel.handshake(:initiator, initiator_opts)
    assert {:ok, resp} = Channel.handshake(:responder, responder_opts)

    assert {:ok, :send, data, init} = Channel.step_handshake(init, {:send, ""})
    assert {:ok, :received, data, resp} = Channel.step_handshake(resp, {:received, data})
    assert {:ok, :send, data, resp} = Channel.step_handshake(resp, {:send, ""})
    assert {:ok, :received, data, init} = Channel.step_handshake(init, {:received, data})
    assert {:ok, :send, data, init} = Channel.step_handshake(init, {:send, ""})
    assert {:ok, :done, init_chan} = Channel.step_handshake(init, :done)
    assert {:ok, :received, data, resp} = Channel.step_handshake(resp, {:received, data})
    assert {:ok, :done, resp_chan} = Channel.step_handshake(resp, :done)

    assert {:ok, init_chan, encrypted} = Channel.encrypt(init_chan, "ping")
    assert {:ok, resp_chan, "ping"} = Channel.decrypt(resp_chan, encrypted)
    assert {:ok, resp_chan, encrypted} = Channel.encrypt(resp_chan, "pong")
    assert {:ok, _init_chan, "pong"} = Channel.decrypt(init_chan, encrypted)
  end

  test "well-known test" do
    # handshake=Noise_XX_25519_AESGCM_SHA256
    s = KeyPair.from_hex("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
    e = KeyPair.from_hex("202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f")
    rs = KeyPair.from_hex("0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20")
    re = KeyPair.from_hex("4142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f60")

    initiator_opts = %{protocol: "Noise_XX_25519_AESGCM_SHA256", s: s, e: e, rs: rs, re: re}
    assert {:ok, init} = Channel.handshake(:initiator, initiator_opts)

    responder_opts = %{protocol: "Noise_XX_25519_AESGCM_SHA256", s: rs, e: re, rs: s, re: e}
    assert {:ok, resp} = Channel.handshake(:responder, responder_opts)

    assert {:ok, :send, data, init} = Channel.step_handshake(init, {:send, ""})
    # msg_0_payload=
    msg_0_ciphertext = "358072d6365880d1aeea329adf9121383851ed21a28e3b75e965d0d2cd166254"
    assert Base.decode16!(msg_0_ciphertext, case: :lower) == data

    assert {:ok, :received, data, resp} = Channel.step_handshake(resp, {:received, data})
    assert {:ok, :send, data, resp} = Channel.step_handshake(resp, {:send, ""})

    # msg_1_payload=
    msg_1_ciphertext =
      "64b101b1d0be5a8704bd078f9895001fc03e8e9f9522f188dd128d9846d484665393019dbd6f438795da206db0886610b26108e424142c2e9b5fd1f7ea70cde8767ce62d7e3c0e9bcefe4ab872c0505b9e824df091b74ffe10a2b32809cab21f"

    assert Base.decode16!(msg_1_ciphertext, case: :lower) == data

    assert {:ok, :received, data, init} = Channel.step_handshake(init, {:received, data})
    assert {:ok, :send, data, init} = Channel.step_handshake(init, {:send, ""})
    assert {:ok, :done, init_chan} = Channel.step_handshake(init, :done)

    # msg_2_payload=
    msg_2_ciphertext =
      "e610eadc4b00c17708bf223f29a66f02342fbedf6c0044736544b9271821ae40e70144cecd9d265dffdc5bb8e051c3f83db32a425e04d8f510c58a43325fbc56"

    assert Base.decode16!(msg_2_ciphertext, case: :lower) == data

    assert {:ok, :received, data, resp} = Channel.step_handshake(resp, {:received, data})
    assert {:ok, :done, resp_chan} = Channel.step_handshake(resp, :done)

    msg_3_payload = Base.decode16!("79656c6c6f777375626d6172696e65", case: :lower)
    msg_3_ciphertext = "9ea1da1ec3bfecfffab213e537ed1791bfa887dd9c631351b3f63d6315ab9a"
    assert {:ok, resp_chan, encrypted} = Channel.encrypt(resp_chan, msg_3_payload)
    assert Base.decode16!(msg_3_ciphertext, case: :lower) == encrypted
    assert {:ok, init_chan, ^msg_3_payload} = Channel.decrypt(init_chan, encrypted)

    # This all depends on the payload, but if we've made it this far, it means everything is good to go
    msg_4_payload = Base.decode16!("7375626d6172696e6579656c6c6f77", case: :lower)
    msg_4_ciphertext = "217c5111fad7afde33bd28abaff3def88a57ab50515115d23a10f28621f842"
    assert {:ok, init_chan, encrypted} = Channel.encrypt(init_chan, msg_4_payload)
    assert Base.decode16!(msg_4_ciphertext, case: :lower) == encrypted
    assert {:ok, _resp_chan, ^msg_4_payload} = Channel.decrypt(resp_chan, encrypted)
  end

  @tag skip: true
  @tag vectors: true
  test "test vectors", %{vectors: vectors} do
    for %{name: name} = vector <- vectors do
      case Protocol.from_name(name) do
        {:ok, protocol} ->
          init_opts = %{
            prologue: fix(Map.get(vector, :init_prologue, "")),
            e: fix(Map.get(vector, :init_ephemeral)),
            s: fix(Map.get(vector, :init_static)),
            rs: fix(Map.get(vector, :init_remote_static))
          }

          resp_opts = %{
            prologue: fix(Map.get(vector, :resp_prologue, "")),
            e: fix(Map.get(vector, :resp_ephemeral)),
            s: fix(Map.get(vector, :resp_static)),
            rs: fix(Map.get(vector, :resp_remote_static))
          }

          messages = Map.get(vector, :messages)
          hash = fix(Map.get(vector, :handshake_hash))

          test_vector(name, protocol, init_opts, resp_opts, messages, hash)

        {:error, {Protocol, :unsupported_pattern}} ->
          :ok
      end
    end
  end

  defp fix(nil), do: nil
  defp fix(bin) when is_binary(bin), do: Base.decode16!(bin, case: :lower)

  defp test_vector(_name, protocol, init, resp, messages, hash) do
    dh = Protocol.dh(protocol)

    secret = fn
      nil -> nil
      sec -> KeyPair.new(dh, private: sec)
    end

    pub = fn
      nil -> nil
      pub -> KeyPair.new(dh, public: pub)
    end

    build_hs = fn p, r, %{e: e, s: s, rs: rs, prologue: pl} ->
      Handshake.init(p, r, pl, {secret.(s), secret.(e), pub.(rs), nil})
    end

    assert {:ok, init_hs} = build_hs.(protocol, :initiator, init)
    assert {:ok, resp_hs} = build_hs.(protocol, :responder, resp)

    test_vector(messages, init_hs, resp_hs, hash)
  end

  defp test_vector([%{payload: pl, ciphertext: ct} = m | msgs], send_hs, recv_hs, hash) do
    pl = Base.decode16!(pl, case: :lower)
    ct = Base.decode16!(ct, case: :lower)

    case {Handshake.next_message(send_hs), Handshake.next_message(recv_hs)} do
      {:out, :in} ->
        assert {:ok, send_hs, message} = Handshake.write_message(send_hs, pl)
        assert ^ct = message
        assert {:ok, recv_hs, pl1} = Handshake.read_message(recv_hs, message)
        assert ^pl = pl1
        test_vector(msgs, recv_hs, send_hs, hash)

      {:done, :done} ->
        {:ok, %Channel{rx: rx1, tx: tx1, hash: hash1} = chan_a} = Handshake.finalize(send_hs)
        {:ok, %Channel{rx: rx2, tx: tx2, hash: hash2} = chan_b} = Handshake.finalize(recv_hs)
        assert ^rx1 = tx2
        assert ^rx2 = tx1
        assert ^hash = hash1
        assert ^hash = hash2
        test_vector([m | msgs], chan_a, chan_b)

      {out_msg, in_msg} ->
        assert {:out, :in} = {out_msg, in_msg}
    end
  end

  defp test_vector([], _, _), do: :ok

  defp test_vector([%{payload: pl, ciphertext: ct} | msgs], ca, cb) do
    pl = Base.decode16!(pl, case: :lower)
    ct = Base.decode16!(ct, case: :lower)
    assert {:ok, new_ca, encrypted} = Channel.encrypt(ca, pl)
    assert ct == encrypted
    assert {:ok, new_cb, decrypted} = Channel.decrypt(cb, encrypted)
    assert pl == decrypted
    test_vector(msgs, new_ca, new_cb)
  end
end
