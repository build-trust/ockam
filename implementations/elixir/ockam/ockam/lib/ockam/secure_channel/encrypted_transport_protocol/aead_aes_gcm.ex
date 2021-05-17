defmodule Ockam.SecureChannel.EncryptedTransportProtocol.AeadAesGcm do
  @moduledoc false

  alias Ockam.Message
  alias Ockam.Router
  alias Ockam.Vault
  alias Ockam.Wire

  def setup(_options, initial_state, data) do
    {:ok, initial_state, data}
  end

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
    message = %{
      payload: Message.payload(message),
      onward_route: Message.onward_route(message) |> List.pop_at(0) |> elem(1),
      return_route: Message.return_route(message)
    }

    with {:ok, encoded} <- Wire.encode(message),
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

    with {:ok, ciphertext} <- Vault.aead_aes_gcm_encrypt(vault, k, n, h, plaintext) do
      data = Map.put(data, :encrypted_transport, %{h: h, decrypt: decrypt, encrypt: {k, n + 1}})
      {:ok, ciphertext, data}
    end
  end

  defp decrypt_and_send_to_router(envelope, state, data) do
    payload = Message.payload(envelope)

    with {:ok, decrypted, data} <- decrypt(payload, data),
         {:ok, decoded} <- Wire.decode(decrypted) do
      message = %{
        payload: Message.payload(decoded),
        onward_route: Message.onward_route(decoded),
        return_route:
          decoded |> Message.return_route() |> List.insert_at(0, data.plaintext_address)
      }

      Router.route(message)

      {:next_state, state, data}
    end
  end

  defp decrypt(ciphertext, %{encrypted_transport: state, vault: vault} = data) do
    %{h: h, decrypt: {k, n}, encrypt: encrypt} = state

    with {:ok, plaintext} <- Vault.aead_aes_gcm_decrypt(vault, k, n, h, ciphertext) do
      data = Map.put(data, :encrypted_transport, %{h: h, decrypt: {k, n + 1}, encrypt: encrypt})
      {:ok, plaintext, data}
    end
  end
end
