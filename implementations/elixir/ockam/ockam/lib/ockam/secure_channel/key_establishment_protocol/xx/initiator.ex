defmodule Ockam.SecureChannel.KeyEstablishmentProtocol.XX.Initiator do
  @moduledoc false

  alias Ockam.Message
  alias Ockam.Router
  alias Ockam.SecureChannel.KeyEstablishmentProtocol.XX.Protocol

  # TODO check for better ways to doing this
  alias Ockam.SecureChannel.EncryptedTransportProtocol.AeadAesGcm, as: EncryptedTransport

  @role :initiator

  def setup(_options, data) do
    {:ok, {:key_establishment, @role, :ready}, data, [{:next_event, :internal, :enter}]}
  end

  def handle_internal(:enter, {:key_establishment, @role, :ready}, data) do
    message1_onward_route = data.peer.route
    message1_return_route = [data.ciphertext_address]

    with {:ok, encoded_message1, data} <- Protocol.encode(:message1, data),
         init_handshake = encode_init_handshake(encoded_message1, data),
         :ok <- send(init_handshake, message1_onward_route, message1_return_route) do
      {:next_state, {:key_establishment, @role, :awaiting_message2}, data}
    else
      {:error, reason} ->
        {:stop, reason}
    end
  end

  def handle_message(message, {:key_establishment, @role, :awaiting_message2}, data) do
    message2_payload = Message.payload(message)
    message3_onward_route = Message.return_route(message)
    message3_return_route = [data.ciphertext_address]

    peer = data.peer
    data = Map.put(data, :peer, %{peer | route: message3_onward_route})

    with {:ok, payload, data} <- Protocol.decode(:message2, message2_payload, data),
         {:ok, data} <- set_peer(payload, data),
         {:ok, encoded_message3, data} <- Protocol.encode(:message3, data),
         :ok <- send(encoded_message3, message3_onward_route, message3_return_route),
         {:ok, data} <- set_encrypted_transport_state(data) do
      {:next_state, {:encrypted_transport, :ready}, data}
    else
      {:error, reason} ->
        {:stop, reason}
    end
  end

  def handle_message(_message, _state, _data) do
    {:stop, :invalid_message_or_state}
  end

  def send(message, onward_route, return_route) do
    envelope = %{onward_route: onward_route, return_route: return_route, payload: message}

    Router.route(envelope)
  end

  def set_peer(address, data) do
    peer =
      Map.get(data, :peer, %{})
      |> Map.put(:public_key, data.xx_key_establishment_state.rs)
      |> Map.put(:plaintext_address, address)

    {:ok, Map.put(data, :peer, peer)}
  end

  def set_encrypted_transport_state(data) do
    with {:ok, {k1, k2, h}, data} <- Protocol.split(data) do
      decrypt_state = EncryptedTransport.Decryptor.new(data.vault, k1, 0)
      encrypt_state = EncryptedTransport.Encryptor.new(data.vault, k2, 0)

      data =
        Map.put(data, :encrypted_transport, %{
          h: h,
          decrypt: decrypt_state,
          encrypt: encrypt_state
        })

      {:ok, data}
    end
  end

  defp encode_init_handshake(payload, data) do
    extra_payload = Map.get(data, :extra_init_payload)
    Ockam.SecureChannel.InitHandshake.encode(%{handshake: payload, extra_payload: extra_payload})
  end
end
