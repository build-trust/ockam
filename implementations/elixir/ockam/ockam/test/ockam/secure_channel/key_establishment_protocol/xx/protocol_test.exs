defmodule Ockam.SecureChannel.KeyEstablishmentProtocol.XX.Protocol.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.SecureChannel.KeyEstablishmentProtocol.XX.Protocol

  alias Ockam.SecureChannel.KeyEstablishmentProtocol.XX.Protocol

  @test_case1 %{
    initiator_static: "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
    initiator_ephemeral: "202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f",
    responder_static: "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20",
    responder_ephemeral: "4142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f60",
    message_1_payload: "",
    message_1_ciphertext: "358072d6365880d1aeea329adf9121383851ed21a28e3b75e965d0d2cd166254",
    message_2_payload: "",
    message_2_ciphertext:
      "64b101b1d0be5a8704bd078f9895001fc03e8e9f9522f188dd128d9846d484665393019dbd6f438795da206db0886610b26108e424142c2e9b5fd1f7ea70cde8767ce62d7e3c0e9bcefe4ab872c0505b9e824df091b74ffe10a2b32809cab21f",
    message_3_payload: "",
    message_3_ciphertext:
      "e610eadc4b00c17708bf223f29a66f02342fbedf6c0044736544b9271821ae40e70144cecd9d265dffdc5bb8e051c3f83db32a425e04d8f510c58a43325fbc56"
  }

  @test_case2 %{
    initiator_static: "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
    initiator_ephemeral: "202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f",
    responder_static: "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20",
    responder_ephemeral: "4142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f60",
    message_1_payload: "746573745f6d73675f30",
    message_1_ciphertext:
      "358072d6365880d1aeea329adf9121383851ed21a28e3b75e965d0d2cd166254746573745f6d73675f30",
    message_2_payload: "746573745f6d73675f31",
    message_2_ciphertext:
      "64b101b1d0be5a8704bd078f9895001fc03e8e9f9522f188dd128d9846d484665393019dbd6f438795da206db0886610b26108e424142c2e9b5fd1f7ea70cde8c9f29dcec8d3ab554f4a5330657867fe4917917195c8cf360e08d6dc5f71baf875ec6e3bfc7afda4c9c2",
    message_3_payload: "746573745f6d73675f32",
    message_3_ciphertext:
      "e610eadc4b00c17708bf223f29a66f02342fbedf6c0044736544b9271821ae40232c55cd96d1350af861f6a04978f7d5e070c07602c6b84d25a331242a71c50ae31dd4c164267fd48bd2"
  }

  def do_test(test_case) do
    test_case =
      test_case
      |> Enum.map(fn {k, v} -> {k, Base.decode16!(v, case: :lower)} end)
      |> Enum.into(%{})

    keypairs = [
      :initiator_static,
      :initiator_ephemeral,
      :responder_static,
      :responder_ephemeral
    ]

    test_case =
      Enum.reduce(keypairs, test_case, fn k, test_case ->
        private_key = Map.get(test_case, k)
        {public_key, ^private_key} = :crypto.generate_key(:ecdh, :x25519, private_key)
        %{test_case | k => %{private: private_key, public: public_key}}
      end)

    {:ok, initiator_state} =
      Protocol.setup(test_case.initiator_static,
        ephemeral_keypair: test_case.initiator_ephemeral,
        payloads: %{message1: test_case.message_1_payload, message3: test_case.message_3_payload}
      )

    {:ok, responder_state} =
      Protocol.setup(test_case.responder_static,
        ephemeral_keypair: test_case.responder_ephemeral,
        payloads: %{message2: test_case.message_2_payload}
      )

    {:ok, message_1_ciphertext, {:continue, initiator_state}} =
      Protocol.out_payload(initiator_state)

    {:ok, {:continue, responder_state}} =
      Protocol.in_payload(responder_state, message_1_ciphertext)

    {:ok, message_2_ciphertext, {:continue, responder_state}} =
      Protocol.out_payload(responder_state)

    {:ok, {:continue, initiator_state}} =
      Protocol.in_payload(initiator_state, message_2_ciphertext)

    {:ok, message_3_ciphertext, {:complete, {k1_i, k2_i, h_i, _rs, p_i}}} =
      Protocol.out_payload(initiator_state)

    {:ok, {:complete, {k1_r, k2_r, h_r, _rs, p_r}}} =
      Protocol.in_payload(responder_state, message_3_ciphertext)

    assert k1_i == k1_r
    assert k2_i == k2_r
    assert h_i == h_r
    assert message_1_ciphertext == test_case.message_1_ciphertext
    assert message_2_ciphertext == test_case.message_2_ciphertext
    assert message_3_ciphertext == test_case.message_3_ciphertext
    assert test_case.message_1_payload == p_r.message1
    assert test_case.message_2_payload == p_i.message2
    assert test_case.message_3_payload == p_r.message3
  end

  test "test cases" do
    assert do_test(@test_case1)
    assert do_test(@test_case2)
  end
end
