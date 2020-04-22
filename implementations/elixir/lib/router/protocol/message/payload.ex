defmodule Ockam.Router.Protocol.Message.Payload do
  use Ockam.Router.Protocol.Message,
    type_id: 2,
    schema: [data: :raw]
end

defmodule Ockam.Router.Protocol.Message.EncryptedPayload do
  use Ockam.Router.Protocol.Message,
    type_id: 3,
    derive: false,
    schema: [data: :raw, tag: :raw]

  defimpl Ockam.Router.Protocol.Encoder do
    def encode(value, opts),
      do: Ockam.Router.Protocol.Encoding.Default.Encoder.encode(value, opts)
  end

  defimpl Ockam.Router.Protocol.Encoding.Default.Encoder do
    alias Ockam.Router.Protocol.Encoding.Helpers
    alias Ockam.Router.Protocol.Message.EncryptedPayload

    def encode(%EncryptedPayload{data: data, tag: tag}, _opts) do
      len = byte_size(data) + byte_size(tag)
      encoded_len = Helpers.encode_leb128_u2(len)
      {:ok, [encoded_len, data, tag]}
    end
  end

  defimpl Ockam.Router.Protocol.Decoder do
    def decode(value, input, opts),
      do: Ockam.Router.Protocol.Encoding.Default.Decoder.decode(value, input, opts)
  end

  defimpl Ockam.Router.Protocol.Encoding.Default.Decoder do
    alias Ockam.Router.Protocol.DecodeError
    alias Ockam.Router.Protocol.Encoding.Helpers
    alias Ockam.Router.Protocol.Message.EncryptedPayload

    def decode(value, input, _opts) do
      {size, rest} = Helpers.decode_leb128_u2(input)
      data_length = size - 16

      if data_length > 0 do
        <<data::binary-size(data_length), tag::binary-size(16), rest::binary>> = rest
        {:ok, %EncryptedPayload{value | data: data, tag: tag}, rest}
      else
        {:error,
         DecodeError.new(
           "invalid payload length, expected at least 48 bytes, but got #{byte_size(input)}"
         )}
      end
    end
  end
end
