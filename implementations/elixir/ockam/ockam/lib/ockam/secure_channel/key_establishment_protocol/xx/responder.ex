defmodule Ockam.SecureChannel.KeyEstablishmentProtocol.XX.Responder do
  @moduledoc false

  alias Ockam.Message
  alias Ockam.Router
  alias Ockam.SecureChannel.KeyEstablishmentProtocol.XX.Protocol

  @role :responder

  def setup(_options, data) do
    actions =
      case Map.get(data, :initiating_message) do
        nil -> []
        message -> [{:next_event, :info, message}]
      end

    {:ok, {:key_establishment, @role, :awaiting_message1}, data, actions}
  end

  def handle_message(message, {:key_establishment, @role, :awaiting_message1}, data) do
    message1_payload = Message.payload(message)
    message2_onward_route = Message.return_route(message)
    message2_return_route = [data.ciphertext_address]

    with {:ok, _payload, data} <- Protocol.decode(:message1, message1_payload, data),
         {:ok, encoded_message2, data} <- Protocol.encode(:message2, data),
         :ok <- send(encoded_message2, message2_onward_route, message2_return_route) do
      {:next_state, {:key_establishment, @role, :awaiting_message3}, data}
    end
  end

  def handle_message(message, {:key_establishment, @role, :awaiting_message3}, data) do
    message3_payload = Message.payload(message)

    peer = data.peer
    data = Map.put(data, :peer, %{peer | route: Message.return_route(message)})

    with {:ok, payload, data} <- Protocol.decode(:message3, message3_payload, data),
         {:ok, data} <- set_peer(payload, data),
         {:ok, data} <- set_encrypted_transport_state(data) do
      {:next_state, {:encrypted_transport, :ready}, data}
    end
  end

  def handle_message(_message, _state, _data) do
    {:error, :invalid_message_or_state}
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
      data = Map.put(data, :encrypted_transport, %{h: h, encrypt: {k1, 0}, decrypt: {k2, 0}})
      {:ok, data}
    end
  end
end
