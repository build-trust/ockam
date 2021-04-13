defmodule Ockam.SecureChannel.KeyEstablishmentProtocol.XX.Initiator do
  @moduledoc false

  alias Ockam.Routable
  alias Ockam.Router
  alias Ockam.SecureChannel.KeyEstablishmentProtocol.XX.Protocol

  @role :initiator

  def setup(_options, data) do
    {:ok, {:key_establishment, @role, :ready}, data, [{:next_event, :info, :enter}]}
  end

  def handle_message(:enter, {:key_establishment, @role, :ready}, data) do
    message1_onward_route = data.route_to_peer
    message1_return_route = [data.ciphertext_address]

    with {:ok, encoded_message1, data} <- Protocol.encode(:message1, data),
         :ok <- send(encoded_message1, message1_onward_route, message1_return_route) do
      {:next_state, {:key_establishment, @role, :awaiting_message2}, data}
    end
  end

  def handle_message(message, {:key_establishment, @role, :awaiting_message2}, data) do
    message2 = Message.payload(message)
    message3_onward_route = Message.return_route(message)
    message3_return_route = [data.ciphertext_address]

    data = Map.put(data, :route_to_peer, message3_onward_route)

    with {:ok, payload, data} <- Protocol.decode(:message2, message2, data),
         {:ok, data} <- set_peer_plaintext_address(payload, data),
         {:ok, encoded_message3, data} <- Protocol.encode(:message3, data),
         :ok <- send(encoded_message3, message3_onward_route, message3_return_route),
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

  def set_peer_plaintext_address(address, data) do
    peer =
      Map.get(data, :peer, %{})
      |> Map.put(:public_key, data.xx_key_establishment_state.rs)
      |> Map.put(:plaintext_address, address)

    {:ok, Map.put(data, :peer, peer)}
  end

  def set_encrypted_transport_state(data) do
    with {:ok, {k1, k2, h}, data} <- Protocol.split(data) do
      data = Map.put(data, :encrypted_transport, %{h: h, decrypt: {k1, 0}, encrypt: {k2, 0}})
      {:ok, data}
    end
  end
end
