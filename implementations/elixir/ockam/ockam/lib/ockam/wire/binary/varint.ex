defmodule Ockam.Wire.Binary.VarInt do
  @moduledoc """
  Encodes and Decodes little-endian, base16, variable length unsigned integers.
  """

  use Bitwise

  alias __MODULE__
  alias Ockam.Wire.DecodeError
  alias Ockam.Wire.EncodeError

  require DecodeError
  require EncodeError

  @range 0..16_383

  @typedoc "Range of encodable integers."
  @type encodable :: 0..16_383

  @doc """
  Encode an integer into a little-endian, base16, variable length unsigned
  integer as a binary.

  Returns the encoded binary if it succeeds.
  Returns `{:error, %EncodeError{}}` if it fails.
  """

  @spec encode(encodable) :: binary | {:error, EncodeError.t()}

  def encode(i) when i >= 0 and i < 128, do: <<0::1, i::unsigned-integer-7>>

  def encode(i) when i >= 128 and i <= 16_383 do
    <<1::1, i &&& 0b01111111::unsigned-integer-7, 0::1,
      i >>> 7 &&& 0b01111111::unsigned-integer-7>>
  end

  def encode(i) do
    error_reason_metadata = [expected_range: @range, argument: i]
    error_reason = {:argument_is_not_an_integer_in_expected_range, error_reason_metadata}
    {:error, EncodeError.new(error_reason)}
  end

  @doc """
  Decode an integer from a little-endian, base16, variable length unsigned
  integer encoded as a binary.

  Returns the decoded integer and the rest of the binary as a tuple.
  Returns `{:error, %DecodeError{}}` if it fails.
  """

  @spec decode(binary | list) :: {encodable, binary} | {:error, DecodeError.t()}

  def decode(data) when is_list(data), do: decode(IO.iodata_to_binary(data))

  def decode(<<0::1, b1::unsigned-integer-7, rest::binary>>), do: {b1, rest}

  def decode(<<1::1, b1::unsigned-integer-7, 0::1, b2::unsigned-integer-7, rest::binary>>),
    do: {(b2 <<< 7) + b1, rest}

  def decode(encoded) do
    error_reason = {:argument_is_not_a_binary_with_expected_structure, [argument: encoded]}
    {:error, DecodeError.new(error_reason)}
  end

  @doc """
  Formats an error returned by #{VarInt}.encode/1 or #{VarInt}.decode/1.

  Returns a string.
  """

  @spec format_error(error :: EncodeError.t() | DecodeError.t()) ::
          formatted_error_message :: String.t()

  def format_error(%EncodeError{reason: reason}) do
    {:argument_is_not_an_integer_in_expected_range, metadata} = reason
    argument = Keyword.get(metadata, :argument)

    """
    #{VarInt}.encode/1 failed because the input argument is not an integer,
    in the expected range:

    Expected range: 0..16_383
    Input: #{argument}
    """
  end

  def format_error(%DecodeError{reason: reason}) do
    {:argument_is_not_a_binary_with_expected_structure, metadata} = reason
    argument = Keyword.get(metadata, :argument)

    """
    #{__MODULE__}.decode/1 failed because the input argument does not have the
    right binary structure:

    Input: #{argument}
    """
  end

  def format_error(error), do: "Unexpected error: #{inspect(error)}"
end
