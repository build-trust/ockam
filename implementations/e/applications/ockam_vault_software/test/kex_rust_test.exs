defmodule Ockam.Kex.Rust.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Kex.Rust

  describe "Ockam.Kex.Rust.init/1" do
    test "can run natively implemented functions" do
      {:ok, initiator_vault_handle} = Ockam.Vault.Software.default_init
      {:ok, initiator_handle} = Ockam.Kex.Rust.kex_init_initiator(initiator_vault_handle)

      {:ok, responder_vault_handle} = Ockam.Vault.Software.default_init
      {:ok, responder_handle} = Ockam.Kex.Rust.kex_init_responder(responder_vault_handle)

      {:ok, m1} = Ockam.Kex.Rust.kex_initiator_encode_message_1(initiator_handle, "payload1");
      :ok = Ockam.Kex.Rust.kex_responder_decode_message_1(responder_handle, m1);

      {:ok, m2} = Ockam.Kex.Rust.kex_responder_encode_message_2(responder_handle, "payload2");
      :ok = Ockam.Kex.Rust.kex_initiator_decode_message_2(initiator_vault_handle, m2);

      {:ok, m3} = Ockam.Kex.Rust.kex_initiator_encode_message_3(initiator_handle, "payload3");
      :ok = Ockam.Kex.Rust.kex_responder_decode_message_3(responder_handle, m3);

      {:ok, initiator} = Ockam.Kex.Rust.kex_initiator_finalize(initiator_handle);
      {:ok, responder} = Ockam.Kex.Rust.kex_responder_finalize(responder_handle);
    end
  end
end
