defmodule Ockam.Message do
  @moduledoc """
  Encodes and decodes messages according to the Ockam on-the-wire binary protocol.
  """

  use Bitwise

  alias Ockam.Message.V1

  defstruct version: 1, onward_route: [], return_route: [], payload: ""

  @doc false
  def decode_varint_u2le(<<0::1, b1::unsigned-integer-7, r::binary>>), do: {b1, r}

  def decode_varint_u2le(
        <<1::1, b1::unsigned-integer-7, 0::1, b2::unsigned-integer-7, r::binary>>
      ),
      do: {(b2 <<< 7) + b1, r}

  @doc false
  def encode_varint_u2le(i) when i >= 0 and i < 128, do: <<0::1, i::unsigned-integer-7>>

  def encode_varint_u2le(i) when i >= 128 and i < 16_384 do
    <<1::1, i &&& 0b01111111::unsigned-integer-7, 0::1,
      i >>> 7 &&& 0b01111111::unsigned-integer-7>>
  end

  def decode(encoded) do
    {version, rest} = decode_varint_u2le(encoded)

    case version do
      1 -> V1.decode(rest)
    end
  end

  def encode(%Ockam.Message{version: version} = message) do
    case version do
      1 -> V1.encode(message)
    end
  end

  defmodule V1 do
    @moduledoc false

    alias Ockam.Message

    def decode_host_address(<<0::8, a::8, b::8, c::8, d::8, rest::binary>>),
      do: {{a, b, c, d}, rest}

    def decode_host_address(
          <<1::8, a::unsigned-little-integer-16, b::unsigned-little-integer-16,
            c::unsigned-little-integer-16, d::unsigned-little-integer-16,
            e::unsigned-little-integer-16, f::unsigned-little-integer-16,
            g::unsigned-little-integer-16, h::unsigned-little-integer-16, rest::binary>>
        ) do
      {{a, b, c, d, e, f, g, h}, rest}
    end

    def decode_host_address(<<2::8, length::8, encoded::binary>>) do
      <<address::binary-size(length), rest::binary>> = encoded
      {address, rest}
    end

    def decode_socket_address(protocol, encoded) do
      {host_address, <<port::unsigned-little-integer-16, rest::binary>>} =
        decode_host_address(encoded)

      {{protocol, {host_address, port}}, rest}
    end

    def decode_address(<<0::8, length::8, encoded::binary>>) do
      <<address::binary-size(length), rest::binary>> = encoded
      {Base.encode16(address, case: :lower), rest}
    end

    def decode_address(<<1::8, _length::8, rest::binary>>), do: decode_socket_address(:tcp, rest)
    def decode_address(<<2::8, _length::8, rest::binary>>), do: decode_socket_address(:udp, rest)

    def decode_addressses(0, addresses, rest), do: {Enum.reverse(addresses), rest}

    def decode_addressses(n, addresses, message) do
      {address, rest} = decode_address(message)
      decode_addressses(n - 1, [address | addresses], rest)
    end

    def decode_route(<<number_of_addresses::unsigned-integer-8, rest::binary>>),
      do: decode_addressses(number_of_addresses, [], rest)

    def decode_payload(<<0::8>>), do: :ping
    def decode_payload(<<1::8>>), do: :pong
    def decode_payload(<<2::8, payload::binary>>) when byte_size(payload) != 0, do: payload
    def decode_payload(<<3::8, rest::binary>> = payload) when byte_size(rest) != 0, do: payload
    def decode_payload(<<4::8, rest::binary>> = payload) when byte_size(rest) != 0, do: payload
    def decode_payload(<<5::8, rest::binary>> = payload) when byte_size(rest) != 0, do: payload

    def decode(encoded) do
      {onward, rest} = decode_route(encoded)
      {return, rest} = decode_route(rest)
      payload = decode_payload(rest)

      %Message{version: 1, onward_route: onward, return_route: return, payload: payload}
    end

    def encode_host_address({a, b, c, d}), do: <<0::8, a::8, b::8, c::8, d::8>>

    def encode_host_address({a, b, c, d, e, f, g, h}) do
      <<1::8, a::unsigned-little-integer-16, b::unsigned-little-integer-16,
        c::unsigned-little-integer-16, d::unsigned-little-integer-16,
        e::unsigned-little-integer-16, f::unsigned-little-integer-16,
        g::unsigned-little-integer-16, h::unsigned-little-integer-16>>
    end

    def encode_address(address) when is_binary(address) do
      {:ok, encoded} = Base.decode16(address, case: :lower)
      <<0::8, byte_size(encoded)::8>> <> encoded
    end

    def encode_address({:tcp, {host, port}}) do
      encoded = encode_host_address(host) <> <<port::unsigned-little-integer-16>>
      <<1::8, byte_size(encoded)::8>> <> encoded
    end

    def encode_address({:udp, {host, port}}) do
      encoded = encode_host_address(host) <> <<port::unsigned-little-integer-16>>
      <<2::8, byte_size(encoded)::8>> <> encoded
    end

    def encode_addresses([], encoded), do: encoded

    def encode_addresses([address | addresses], encoded),
      do: encode_addresses(addresses, encoded <> encode_address(address))

    def encode_route(addresses) do
      number_of_addresses = length(addresses)
      encode_addresses(addresses, <<number_of_addresses::unsigned-integer-8>>)
    end

    def encode_payload(:ping), do: <<0::8>>
    def encode_payload(:pong), do: <<1::8>>
    def encode_payload(<<3::8, _rest::binary>> = payload), do: payload
    def encode_payload(<<4::8, _rest::binary>> = payload), do: payload
    def encode_payload(<<5::8, _rest::binary>> = payload), do: payload
    def encode_payload(payload) when is_binary(payload) and byte_size(payload) > 0,
      do: <<2::8>> <> payload

    def encode(%Message{onward_route: onward, return_route: return, payload: payload}) do
      onward_route = encode_route(onward)
      return_route = encode_route(return)

      Message.encode_varint_u2le(1) <> onward_route <> return_route <> encode_payload(payload)
    end
  end
end
