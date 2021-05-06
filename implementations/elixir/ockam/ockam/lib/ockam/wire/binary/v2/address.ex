defmodule Ockam.Wire.Binary.V2.Address do
  @moduledoc false

  alias Ockam.Serializable
  alias Ockam.Transport.TCPAddress
  alias Ockam.Transport.UDPAddress
  alias Ockam.Wire.DecodeError
  alias Ockam.Wire.EncodeError

  require DecodeError
  require EncodeError

  # TODO: this should be pluggable
  @tcp 1
  @udp 2
  if Code.ensure_loaded?(Ockam.Kafka.Transport.Address) do
    @kafka 3
  end

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

  def decode(%{type: 0, value: value}), do: value
  def decode(%{type: @tcp, value: value}), do: TCPAddress.deserialize(value)
  def decode(%{type: @udp, value: value}), do: UDPAddress.deserialize(value)

  if Code.ensure_loaded?(Ockam.Kafka.Transport.Address) do
    def decode(%{type: @kafka, value: value}),
      do: Ockam.Kafka.Transport.Address.deserialize(value)
  end

  def decode(encoded) do
    reason = {:could_not_decode_address, encoded}
    {:error, DecodeError.new(reason)}
  end
end
