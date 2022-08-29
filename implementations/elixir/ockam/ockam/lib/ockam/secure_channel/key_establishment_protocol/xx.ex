defmodule Ockam.SecureChannel.KeyEstablishmentProtocol.XX do
  @moduledoc false

  alias Ockam.SecureChannel.KeyEstablishmentProtocol.XX.Initiator
  alias Ockam.SecureChannel.KeyEstablishmentProtocol.XX.Protocol
  alias Ockam.SecureChannel.KeyEstablishmentProtocol.XX.Responder

  def setup(options, data) do
    options = Keyword.put(options, :message2_payload, data.plaintext_address)
    options = Keyword.put(options, :message3_payload, data.plaintext_address)

    with {:ok, data} <- Protocol.setup(options, data) do
      case Keyword.get(options, :role, :initiator) do
        :initiator -> Initiator.setup(options, data)
        :responder -> Responder.setup(options, data)
        unexpected_role -> {:error, {:role_option_has_an_unexpected_value, unexpected_role}}
      end
    end
  end

  def handle_internal(event, {:key_establishment, role, _role_state} = state, data)
      when role in [:initiator, :responder] do
    case role do
      :initiator -> Initiator.handle_internal(event, state, data)
      :responder -> Responder.handle_internal(event, state, data)
    end
  end

  ## TODO: batter name to not collide with Ockam.Worker.handle_message
  def handle_message(message, {:key_establishment, role, _role_state} = state, data)
      when role in [:initiator, :responder] do
    case role do
      :initiator -> Initiator.handle_message(message, state, data)
      :responder -> Responder.handle_message(message, state, data)
    end
  end
end
