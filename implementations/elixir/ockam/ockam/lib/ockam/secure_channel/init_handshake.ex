defmodule Ockam.SecureChannel.InitHandshake do
  @moduledoc """
  Payload of the first message sent by secure channel initiator
  to secure channel listener

  Contains
  - handshake: binary - first handshake payload
  - extra_payload: optional(binary) - additional information for the listener
  """

  require Logger

  @schema {:struct, [handshake: :data, extra_payload: {:optional, :data}]}

  def decode(binary) do
    case :bare.decode(binary, @schema) do
      {:ok, %{handshake: handshake_bytes, extra_payload: extra_payload}, extra_data} ->
        extra_payload =
          case extra_data do
            "" ->
              extra_payload

            _binary ->
              ## Unable to parse extra_payload: old version use a different data structure there
              Logger.warn("Cannot parse extra payload: ignoring")
              nil
          end

        ## TODO: optimise double encoding of binaries
        handshake = :bare.encode(handshake_bytes, :data)

        {:ok, %{handshake: handshake, extra_payload: extra_payload}}

      {:error, reason} ->
        {:error, reason}
    end
  end

  def encode(%{handshake: handshake, extra_payload: extra_payload}) do
    extra_payload =
      case extra_payload do
        nil -> :undefined
        other -> other
      end

    ## TODO: optimise double encoding of binaries
    {:ok, handshake_bytes, ""} = :bare.decode(handshake, :data)

    :bare.encode(%{handshake: handshake_bytes, extra_payload: extra_payload}, @schema)
  end
end
