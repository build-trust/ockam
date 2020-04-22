defmodule Ockam.Router.Protocol.Encoding.Helpers do
  import Bitwise

  alias Ockam.Router.Protocol.EncodeError
  alias Ockam.Router.Protocol.DecodeError

  @doc """
  Encodes a signed/unsigned integer using a one-byte encoding

  If the value requires more than one byte to be encoded, this
  function will throw `EncodeError`
  """
  def encode_i1(v, _opts) when is_integer(v) and v > -128 and v <= 255 do
    <<v::8>>
  end

  def encode_i1(v, _opts), do: throw(EncodeError.new({:invalid_i1, v}))

  @doc """
  Encodes an unsigned integer using a variable length encoding (LEB-128)
  which is constrained to a maximum of two bytes.

  If the value requires more than two bytes to be encoded, this function
  will throw `EncodeError`
  """
  @spec encode_leb128_u2(non_neg_integer()) :: binary | no_return
  def encode_leb128_u2(v)

  def encode_leb128_u2(v) when is_integer(v) and v <= 16384,
    do: do_encode_leb128(v)

  def encode_leb128_u2(v) do
    throw(EncodeError.new({:invalid_leb128, v}))
  end

  @doc """
  Encodes an unsigned integer using a variable length encoding (LEB-128)

  This function will throw `EncodeError` if given a non-integer value
  """
  @spec encode_leb128(non_neg_integer()) :: binary
  def encode_leb128(v)

  def encode_leb128(v) when is_integer(v),
    do: do_encode_leb128(v)

  def encode_leb128(v),
    do: throw(EncodeError.new({:invalid_leb128, v}))

  defp do_encode_leb128(v) when is_integer(v) and v < 128, do: <<0::1, v::7>>

  defp do_encode_leb128(v) when is_integer(v),
    do: <<1::1, v::7, do_encode_leb128(v >>> 7)::binary>>

  @doc """
  Decodes an unsigned integer using a variable length encoding (LEB-128)
  which is constrained to a maximum of two bytes

  If the decoded value would exceed two bytes, this function
  will throw `DecodeError`
  """
  @spec decode_leb128_u2(binary) :: non_neg_integer() | no_return
  def decode_leb128_u2(bin)

  def decode_leb128_u2(<<0::1, byte::7, rest::binary>>) do
    {byte, rest}
  end

  def decode_leb128_u2(<<1::1, lo::7, 0::1, hi::7, rest::binary>>) do
    {lo ||| hi <<< 7, rest}
  end

  def decode_leb128_u2(input) do
    throw(DecodeError.new({:invalid_leb128_u2, input}))
  end

  @doc """
  Decodes an unsigned integer using a variable length encoding (LEB-128)

  This function will throw `EncodeError` if given a non-binary value
  """
  @spec decode_leb128(binary) :: non_neg_integer()
  def decode_leb128(bin) do
    do_decode_leb128(bin)
  catch
    :throw, :invalid_leb128 ->
      throw(DecodeError.new({:invalid_leb128, bin}))
  end

  defp do_decode_leb128(v, shift \\ 0, result \\ 0)

  defp do_decode_leb128(<<0::1, byte::7, rest::binary>>, 0, 0) do
    {byte, rest}
  end

  defp do_decode_leb128(<<0::1, byte::7, rest::binary>>, shift, result) do
    {result ||| byte <<< shift, rest}
  end

  defp do_decode_leb128(<<1::1, byte::7, rest::binary>>, shift, result) do
    do_decode_leb128(
      rest,
      shift + 7,
      result ||| byte <<< shift
    )
  end

  defp do_decode_leb128(_v, _shift, _result), do: throw(:invalid_leb128)
end
