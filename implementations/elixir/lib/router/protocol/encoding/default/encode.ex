defmodule Ockam.Router.Protocol.Encoding.Default.Encode do
  @moduledoc """
  Encodes values using the `Default` encoder.
  """

  import Ockam.Router.Protocol.Encoding.Helpers

  alias Ockam.Router.Protocol.EncodeError
  alias Ockam.Router.Protocol.Encoding.Default
  alias Ockam.Router.Protocol.Encoding.Default.Encoder

  @opaque opts :: map()

  @doc false
  @spec encode(any, opts) :: {:ok, iodata} | {:error, EncodeError.t() | Exception.t()}
  def encode(value, opts) do
    try do
      {:ok, encode_value(value, opts)}
    catch
      :throw, %EncodeError{} = e ->
        {:error, e}

      :error, %Protocol.UndefinedError{protocol: Default.Encoder} = e ->
        {:error, e}
    end
  end

  @doc false
  @spec encode_value(any, opts) :: iodata | no_return
  def encode_value(value, opts)

  def encode_value(%{__struct__: _} = value, opts) do
    with {:ok, encoded} <- Encoder.encode(value, opts) do
      encoded
    else
      {:error, err} ->
        throw(err)
    end
  end

  def encode_value(value, opts) when is_map(value) do
    map(value, opts)
  end

  def encode_value(value, opts) when is_list(value) do
    list(value, opts)
  end

  def encode_value(value, opts) do
    with {:ok, encoded} <- Encoder.encode(value, opts) do
      encoded
    else
      {:error, err} ->
        throw(err)
    end
  end

  def atom(true, opts), do: boolean(true, opts)
  def atom(false, opts), do: boolean(false, opts)
  def atom(nil, _opts), do: <<>>
  def atom(value, opts), do: string(Atom.to_string(value), opts)

  def raw(s, _opts), do: [encode_size(s), s]
  def string(s, _opts), do: [encode_size(s), s]
  def iso8601_string(s, _opts), do: [encode_size(s), s]

  def boolean(true, _opts), do: <<1::8>>
  def boolean(false, _opts), do: <<0::8>>

  def i1(value, opts), do: encode_i1(value, opts)

  def integer(value, _opts) when is_integer(value) do
    encode_leb128(value)
  end

  def float(value, _opts) when is_float(value) do
    <<int::64>> = <<value::float>>
    encode_leb128(int)
  end

  def list(value, opts) when is_list(value) do
    [encode_size(value) | do_encode_list(value, opts)]
  end

  defp do_encode_list([], _opts), do: []

  defp do_encode_list([h | t], opts) do
    [encode_value(h, opts) | do_encode_list(t, opts)]
  end

  def map(value, opts) do
    [encode_size(value) | do_encode_map(Map.to_list(value), opts)]
  end

  defp do_encode_map([], _opts), do: []

  defp do_encode_map([{key, value} | t], opts) do
    [encode_value(key, opts), encode_value(value, opts) | do_encode_map(t, opts)]
  end

  def encode_size(map) when is_map(map), do: encode_leb128_u2(map_size(map))
  def encode_size(list) when is_list(list), do: encode_leb128_u2(length(list))
  def encode_size(bin) when is_binary(bin), do: encode_leb128_u2(byte_size(bin))
end
