defmodule Ockam.SecureChannel.EncryptedTransportProtocol.AeadAesGcm do
  @moduledoc false

  alias Ockam.Message
  alias Ockam.Router
  alias Ockam.Vault
  alias Ockam.Wire

  def setup(_options, initial_state, data) do
    {:ok, initial_state, data}
  end

  ## TODO: batter name to not collide with Ockam.Worker.handle_message
  def handle_message(message, {:encrypted_transport, :ready} = state, data) do
    first_address = message |> Message.onward_route() |> List.first()

    cond do
      first_address === data.ciphertext_address ->
        decrypt_and_send_to_router(message, state, data)

      first_address === data.plaintext_address ->
        encrypt_and_send_to_peer(message, state, data)

      true ->
        {:next_state, state, data}
    end
  end

  defp encrypt_and_send_to_peer(message, state, data) do
    forwarded_message = Message.forward(message)

    with {:ok, encoded} <- Wire.encode(forwarded_message),
         {:ok, encrypted, data} <- encrypt(encoded, data) do
      envelope = %{
        payload: encrypted,
        onward_route: data.peer.route,
        return_route: [data.ciphertext_address]
      }

      Router.route(envelope)

      {:next_state, state, data}
    end
  end

  defp encrypt(plaintext, %{encrypted_transport: state, vault: vault} = data) do
    %{h: h, decrypt: decrypt, encrypt: {k, n}} = state

    with {:ok, ciphertext} <- Vault.aead_aes_gcm_encrypt(vault, k, n, h, plaintext),
         {:ok, next_n} <- increment_nonce(n) do
      data = Map.put(data, :encrypted_transport, %{h: h, decrypt: decrypt, encrypt: {k, next_n}})
      {:ok, <<n::unsigned-big-integer-size(16)>> <> ciphertext, data}
    end
  end

  defp decrypt_and_send_to_router(envelope, state, data) do
    payload = Message.payload(envelope)

    with {:ok, decrypted, data} <- decrypt(payload, data),
         {:ok, decoded} <- Wire.decode(decrypted) do
      message = Message.trace(decoded, data.plaintext_address)

      Router.route(message)

      {:next_state, state, data}
    end
  end

  defp decrypt(<<n::unsigned-big-integer-size(16), ciphertext::binary>>, data) do
    %{encrypted_transport: state, vault: vault} = data
    %{h: h, decrypt: {k, _expected_n}, encrypt: encrypt} = state

    with {:ok, plaintext} <- Vault.aead_aes_gcm_decrypt(vault, k, n, h, ciphertext),
         {:ok, next_expected_n} <- increment_nonce(n) do
      data =
        Map.put(data, :encrypted_transport, %{
          h: h,
          decrypt: {k, next_expected_n},
          encrypt: encrypt
        })

      {:ok, plaintext, data}
    end
  end

  # TODO: we can reuse a nonse, we must rotate keys
  defp increment_nonce(65_535), do: {:error, nil}
  defp increment_nonce(n), do: {:ok, n + 1}
end
