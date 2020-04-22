defmodule Ockam.Router.Protocol.Message.Envelope do
  defstruct headers: %{}, body: nil

  defimpl Ockam.Router.Protocol.Encoder do
    def encode(value, opts),
      do: Ockam.Router.Protocol.Encoding.Default.encode(value, opts)
  end

  defimpl Ockam.Router.Protocol.Encoding.Default.Encoder do
    alias Ockam.Router.Protocol.Encoding.Default

    def encode(value, opts) do
      Default.encode(value, opts)
    end
  end

  defimpl Ockam.Router.Protocol.Decoder do
    def decode(value, input, opts),
      do: Ockam.Router.Protocol.Encoding.Default.Decoder.decode(value, input, opts)
  end

  defimpl Ockam.Router.Protocol.Encoding.Default.Decoder do
    alias Ockam.Router.Protocol.DecodeError
    alias Ockam.Router.Protocol.Encoding.Default.Decoder
    alias Ockam.Router.Protocol.Encoding.Helpers
    alias Ockam.Router.Protocol.Endpoint
    alias Ockam.Router.Protocol.Message
    alias Ockam.Router.Protocol.Message.Envelope

    def decode(value, input, opts) do
      {headers_len, rest} = Helpers.decode_leb128_u2(input)

      with {:ok, headers, rest} <- decode_headers(headers_len, rest, opts),
           {:ok, type, rest} <- decode_message_type(rest),
           {:ok, body, rest} <- Decoder.decode(struct(type, []), rest, opts) do
        {:ok, %Envelope{value | headers: headers, body: body}, rest}
      end
    end

    defp decode_headers(n, input, opts), do: decode_headers(n, input, opts, %{})

    defp decode_headers(0, input, _opts, acc), do: {:ok, acc, input}

    defp decode_headers(n, <<type::8, rest::binary>>, opts, acc) do
      case type do
        type when type in [0, 1] ->
          key = if type == 0, do: :send_to, else: :reply_to

          with {:ok, endpoint, rest} <- Decoder.decode(%Endpoint{}, rest, opts) do
            decode_headers(n - 1, rest, opts, Map.put(acc, key, endpoint))
          end

        unknown ->
          {:error, DecodeError.new("unknown header type (#{inspect(unknown)})")}
      end
    end

    defp decode_message_type(<<ty::8, input::binary>>) do
      with {:ok, type} = Message.lookup(ty) do
        {:ok, type, input}
      end
    end
  end
end
