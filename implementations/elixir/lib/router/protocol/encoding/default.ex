defmodule Ockam.Router.Protocol.Encoding.Default do
  @behaviour Ockam.Router.Protocol.Encoding

  alias __MODULE__.Encode
  alias __MODULE__.Encoder
  alias __MODULE__.Decoder
  alias Ockam.Router.Protocol.DecodeError
  alias Ockam.Router.Protocol.Encoding
  alias Ockam.Router.Protocol.Encoding.Helpers
  alias Ockam.Router.Protocol.Message.Envelope

  @type message :: Encoding.message()
  @type opts :: Encoding.opts()
  @type reason :: Encoding.reason()

  @type keys :: :atoms | :atoms! | :strings | :copy | (String.t() -> term)
  @type strings :: :reference | :copy
  @type decode_opt :: {:keys, keys} | {:strings, strings}

  @spec encode!(message, opts) :: iodata | no_return
  def encode!(message, opts \\ %{})

  def encode!(message, opts) do
    case encode(message, opts) do
      {:ok, encoded} ->
        encoded

      {:error, error} ->
        raise error
    end
  end

  @spec encode(message, opts) :: {:ok, iodata} | {:error, reason}
  def encode(message, opts \\ %{})

  def encode(%Envelope{headers: headers, body: %type{} = body}, opts) do
    opts = normalize_encode_opts(opts)
    headers_len = Helpers.encode_leb128_u2(map_size(headers))

    with {:ok, headers} <- encode_headers(Map.to_list(headers), opts),
         {:ok, body} <- Encode.encode(body, opts) do
      type_id = Helpers.encode_leb128_u2(type.type_id())
      version = Helpers.encode_leb128_u2(1)
      {:ok, IO.iodata_to_binary([version, headers_len, headers, type_id, body])}
    end
  end

  def encode(%_{} = message, opts) when is_map(opts) do
    encode(%Envelope{body: message}, opts)
  end

  defp encode_headers(headers, opts), do: encode_headers(headers, opts, [])

  defp encode_headers([], _opts, acc), do: {:ok, acc}

  defp encode_headers([header | rest], opts, acc) do
    with {:ok, {type, value}} <- header_to_header_type(header),
         {:ok, encoded} <- Encoder.encode(value, opts) do
      encode_headers(rest, opts, [<<type::8>>, encoded | acc])
    end
  end

  defp header_to_header_type({:send_to, endpoint}), do: {:ok, {0, endpoint}}
  defp header_to_header_type({:reply_to, endpoint}), do: {:ok, {1, endpoint}}
  defp header_to_header_type({key, _value}), do: {:error, {:unknown_header_type, key}}

  @spec decode!(iodata, opts) :: {message, iodata} | no_return
  def decode!(input, opts \\ %{})

  def decode!(input, opts) do
    case decode(input, opts) do
      {:ok, message, rest} ->
        {message, rest}

      {:error, error} ->
        raise error
    end
  end

  @spec decode(iodata, opts) :: {:ok, message, iodata} | {:error, reason}
  def decode(input, opts \\ %{})

  def decode(input, opts) when (is_binary(input) or is_list(input)) and is_map(opts) do
    input = IO.iodata_to_binary(input)

    with {:ok, {version, input}} <- decode_version(input) do
      decode_envelope(version, input, normalize_decode_opts(opts))
    end
  end

  defp decode_version(input) do
    {:ok, Helpers.decode_leb128_u2(input)}
  catch
    :throw, err ->
      {:error, err}
  end

  defp decode_envelope(1, input, opts) do
    Decoder.decode(%Envelope{}, input, opts)
  end

  defp decode_envelope(version, _input, _opts) do
    {:error, DecodeError.new("unknown message version (#{inspect(version)})")}
  end

  defp normalize_encode_opts(opts), do: opts

  defp normalize_decode_opts(opts) do
    Enum.into(opts, %{keys: :strings, strings: :reference})
  end
end
