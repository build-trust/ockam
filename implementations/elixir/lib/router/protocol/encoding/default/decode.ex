defmodule Ockam.Router.Protocol.Encoding.Default.Decode do
  @moduledoc """
  Decodes encoded values using the `Default` decoder.
  """

  import Ockam.Router.Protocol.Encoding.Helpers

  alias Ockam.Router.Protocol.DecodeError
  alias Ockam.Router.Protocol.Encoding.Default.Decoder

  @opaque opts :: map()
  @type type ::
          :raw
          | :string
          | :atom
          | :integer
          | :float
          | module()
  @type message :: term()

  @doc false
  @spec decode(type, iodata, opts) ::
          {:ok, message, iodata} | {:error, DecodeError.t() | Exception.t()}
  def decode(type, encoded, opts) when is_binary(encoded) do
    keys_fun = key_decoder(opts)
    strings_fun = string_decoder(opts)

    decode(type, encoded, keys_fun, strings_fun, opts)
  end

  @doc false
  def decode(type, encoded, keys_fun, strings_fun, opts) when is_binary(encoded) do
    {decoded, rest} = decode_type(type, encoded, keys_fun, strings_fun, opts)
    {:ok, decoded, rest}
  catch
    :throw, %DecodeError{} = e ->
      {:error, e}
  end

  def decode_list_of(0, acc, _type, input, _keys_fun, _strings_fun, _opts),
    do: {Enum.reverse(acc), input}

  def decode_list_of(n, acc, type, input, keys_fun, strings_fun, opts) do
    {decoded, rest} = decode_type(type, input, keys_fun, strings_fun, opts)
    decode_list_of(n - 1, [decoded | acc], type, rest, keys_fun, strings_fun, opts)
  end

  def decode_type([type], input, keys_fun, strings_fun, opts) do
    {num_entries, rest} = decode_leb128(input)
    decode_list_of(num_entries, [], type, rest, keys_fun, strings_fun, opts)
  end

  def decode_type(:raw, input, _keys_fun, strings_fun, _opts) do
    decode_raw(input, strings_fun)
  end

  def decode_type(:string, input, _keys_fun, strings_fun, _opts) do
    decode_raw(input, strings_fun)
  end

  def decode_type(:atom, input, keys_fun, strings_fun, _opts) do
    {raw, rest} = decode_raw(input, strings_fun)
    {keys_fun.(raw), rest}
  end

  def decode_type(:i1, <<value::8, rest::binary>>, _, _, _), do: {value, rest}
  def decode_type(:integer, input, _, _, _), do: decode_leb128(input)

  def decode_type(:float, input, _keys_fun, _strings_fun, _opts) do
    {bytes, rest} = decode_leb128(input)

    case <<bytes::binary>> do
      <<f::float>> ->
        {f, rest}

      _ ->
        throw(DecodeError.new({:type_error, {:float, bytes}}))
    end
  end

  def decode_type(:boolean, <<1::8, rest::binary>>, _, _, _), do: {true, rest}
  def decode_type(:boolean, <<0::8, rest::binary>>, _, _, _), do: {false, rest}

  def decode_type(:boolean, <<byte::8, _::binary>>, _, _, _) do
    throw(DecodeError.new({:type_error, {nil, <<byte::8>>}}))
  end

  def decode_type(module, input, _keys_fun, _strings_fun, opts) when is_atom(module) do
    with {:ok, decoded, rest} <- Decoder.decode(struct(module, []), input, opts) do
      {decoded, rest}
    else
      {:error, err} ->
        throw(err)
    end
  end

  def decode_type(unknown, _input, _keys_fun, _strings_fun, _opts) do
    throw(DecodeError.new({:invalid_type, unknown}))
  end

  def decode_raw(input, strings_fun) do
    {len, rest} = decode_leb128(input)

    case rest do
      <<s::binary-size(len), rest::binary>> ->
        {strings_fun.(s), rest}

      s ->
        throw(DecodeError.new({:unexpected_eof, len, byte_size(s)}))
    end
  end

  def key_decoder(%{keys: :atoms}), do: &String.to_atom/1
  def key_decoder(%{keys: :atoms!}), do: &String.to_existing_atom/1
  def key_decoder(%{keys: :strings}), do: & &1
  def key_decoder(%{keys: fun}) when is_function(fun, 1), do: fun
  def key_decoder(%{}), do: & &1

  def string_decoder(%{strings: :copy}), do: &:binary.copy/1
  def string_decoder(%{strings: :reference}), do: & &1
  def string_decoder(%{}), do: & &1
end
