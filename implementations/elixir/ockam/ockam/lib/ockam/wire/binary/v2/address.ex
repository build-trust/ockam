defmodule Ockam.Wire.Binary.V2.Address do
  @moduledoc false

  alias Ockam.Serializable
  alias Ockam.Wire.DecodeError
  alias Ockam.Wire.EncodeError

  require DecodeError
  require EncodeError

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
      serialized -> serialized |> IO.inspect
    end
  end

  def decode(<<address_type::unsigned-integer-8, length::8, encoded::binary>>) do
    case encoded do
      <<value::binary-size(length), rest::binary>> ->
        address = <<address_type::unsigned-integer-8, length::8, value::binary-size(length)>>
        {{address_type, address}, rest}

      _else ->
        reason = {:could_not_decode_address_of_detected_length, length, encoded}
        {:error, DecodeError.new(reason)}
    end
  end

  def decode(encoded) do
    reason = {:could_not_decode_address, encoded}
    {:error, DecodeError.new(reason)}
  end
end
