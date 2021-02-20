defmodule Ockam.Wire.Binary.V2.Address do
  @moduledoc false

  alias Ockam.Serializable
  alias Ockam.Transport.TCPAddress
  alias Ockam.Transport.UDPAddress
  alias Ockam.Wire.DecodeError
  alias Ockam.Wire.EncodeError

  require DecodeError
  require EncodeError

  # TODO: this should be refactored
  @tcp 1
  @udp 2

  @spec encode(any) :: binary | maybe_improper_list() | {:error, Ockam.Wire.EncodeError.t()}
  def encode({_address_type, address}) when is_binary(address) do
    address
  end

  def encode(address) do
    case Serializable.impl_for(address) do
      nil ->
        reason = {:address_is_not_serializable, address}
        {:error, EncodeError.new(reason)}

      _impl ->
        encode_serializable_address(address)
    end
  end

  defp encode_serializable_address(address) do
    case Serializable.serialize(address) do
      {:error, reason} -> {:error, EncodeError.new({reason, address})}
      serialized -> serialized
    end
  end

  def decode(%{type: type, value: value}) do
    # TODO: there needs to be a way to do this programmatically
    case type do
      @tcp -> TCPAddress.deserialize(value)
      @udp -> UDPAddress.deserialize(value)
      0 -> value
    end
  end

  def decode(encoded) do
    reason = {:could_not_decode_address, encoded}
    {:error, DecodeError.new(reason)}
  end
end
