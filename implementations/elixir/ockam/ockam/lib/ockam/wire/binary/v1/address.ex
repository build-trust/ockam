defmodule Ockam.Wire.Binary.V1.Address do
  @moduledoc false

  alias Ockam.Address
  alias Ockam.Serializable
  alias Ockam.Wire.DecodeError
  alias Ockam.Wire.EncodeError

  require DecodeError
  require EncodeError

  def encode(address) do
    type = Address.type(address)
    value = Address.value(address)

    case Serializable.impl_for(value) do
      nil ->
        reason = {:address_value_is_not_serializable, value}
        {:error, EncodeError.new(reason)}

      _impl ->
        encode_serializable_address(type, value)
    end
  end

  defp encode_serializable_address(type, value) do
    case Serializable.serialize(value) do
      {:error, reason} -> {:error, EncodeError.new({reason, value})}
      serialized -> [<<type::8>>, <<byte_size(serialized)::8>>, serialized]
    end
  end

  def decode(<<address_type::unsigned-integer-8, length::8, encoded::binary>>) do
    case encoded do
      <<address::binary-size(length), rest::binary>> ->
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
