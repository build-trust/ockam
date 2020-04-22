defmodule Ockam.Router.Protocol.Endpoint do
  alias Ockam.Transport.Address

  defstruct [:value]

  defmodule Local do
    defstruct [:data]

    def type_id(%__MODULE__{}), do: 0

    defimpl Ockam.Router.Protocol.Encoder do
      def encode(value, opts),
        do: Ockam.Router.Protocol.Encoding.Default.Encoder.encode(value, opts)
    end

    defimpl Ockam.Router.Protocol.Encoding.Default.Encoder do
      alias Ockam.Router.Protocol.Encoding.Helpers
      alias Ockam.Router.Protocol.Encoding.Default.Encode
      alias Ockam.Router.Protocol.Encoding.Default.Encoder
      alias Ockam.Router.Protocol.Endpoint.Local

      def encode(%Local{data: data} = value, opts) do
        type = Encode.i1(Local.type_id(value), opts)
        len = Helpers.encode_leb128_u2(byte_size(data))
        {:ok, [type, len, data]}
      end
    end

    defimpl Ockam.Router.Protocol.Decoder do
      def decode(value, input, opts),
        do: Ockam.Router.Protocol.Encoding.Default.Decoder.decode(value, input, opts)
    end

    defimpl Ockam.Router.Protocol.Encoding.Default.Decoder do
      alias Ockam.Router.Protocol.Encoding.Helpers
      alias Ockam.Router.Protocol.Endpoint.Local

      def decode(value, input, _opts) do
        {len, rest} = Helpers.decode_leb128_u2(input)

        if len > 0 do
          <<data::binary-size(len), rest::binary>> = rest
          {:ok, %Local{value | data: data}, rest}
        else
          {:ok, value, rest}
        end
      end
    end
  end

  defmodule Channel do
    defstruct [:public_key, :endpoint]

    def type_id(%__MODULE__{}), do: 1

    defimpl Ockam.Router.Protocol.Encoder do
      def encode(value, opts),
        do: Ockam.Router.Protocol.Encoding.Default.Encoder.encode(value, opts)
    end

    defimpl Ockam.Router.Protocol.Encoding.Default.Encoder do
      alias Ockam.Router.Protocol.Encoding.Default.Encode
      alias Ockam.Router.Protocol.Encoding.Default.Encoder
      alias Ockam.Router.Protocol.Endpoint.Channel

      def encode(%Channel{public_key: pk, endpoint: endpoint} = value, opts) do
        with {:ok, endpoint_encoded} <- Encoder.encode(endpoint, opts) do
          type = Encode.i1(Channel.type_id(value), opts)
          {:ok, pk_encoded} = encode_public_key(pk)
          {:ok, [type, pk_encoded, endpoint_encoded]}
        end
      end

      defp encode_public_key({:x25519, <<pk::binary-size(32)>>}),
        do: <<1::8, pk::binary>>

      defp encode_public_key({:p256_compressed_y0, <<pk::binary-size(32)>>}),
        do: <<2::8, pk::binary>>

      defp encode_public_key({:p256_compressed_y1, <<pk::binary-size(32)>>}),
        do: <<3::8, pk::binary>>

      defp encode_public_key({
             :p256_uncompressed,
             <<x::binary-size(32)>>,
             <<y::binary-size(32)>>
           }) do
        <<4::8, x::binary, y::binary>>
      end
    end

    defimpl Ockam.Router.Protocol.Decoder do
      def decode(value, input, opts),
        do: Ockam.Router.Protocol.Encoding.Default.Decoder.decode(value, input, opts)
    end

    defimpl Ockam.Router.Protocol.Encoding.Default.Decoder do
      alias Ockam.Router.Protocol.DecodeError
      alias Ockam.Router.Protocol.Encoding.Default.Decoder
      alias Ockam.Router.Protocol.Endpoint
      alias Ockam.Router.Protocol.Endpoint.Channel

      def decode(value, input, opts) do
        with {:ok, pk, rest} <- decode_public_key(input, opts),
             {:ok, endpoint, rest} <- Decoder.decode(%Endpoint{}, rest, opts) do
          {:ok, %Channel{value | public_key: pk, endpoint: endpoint}, rest}
        end
      end

      defp decode_public_key(<<type::8, rest::binary>>, opts),
        do: decode_public_key(type, rest, opts)

      defp decode_public_key(1, <<pk::size(32)-unit(8), rest::binary>>, _opts),
        do: {:ok, {:x25519, pk}, rest}

      defp decode_public_key(2, <<pk::size(32)-unit(8), rest::binary>>, _opts),
        do: {:ok, {:p256_compressed_y0, pk}, rest}

      defp decode_public_key(3, <<pk::size(32)-unit(8), rest::binary>>, _opts),
        do: {:ok, {:p256_compressed_y1, pk}, rest}

      defp decode_public_key(
             4,
             <<x::size(32)-unit(8), y::size(32)-unit(8), rest::binary>>,
             _opts
           ),
           do: {:ok, {:p256_uncompressed, x, y}, rest}

      defp decode_public_key(type, input, _opts) do
        {:error,
         DecodeError.new(
           "invalid public key (type = #{inspect(type)}, input = #{inspect(input)})"
         )}
      end
    end
  end

  defmodule IPv4 do
    defstruct [:protocol, :address]

    def type_id(%__MODULE__{protocol: :tcp}), do: 2
    def type_id(%__MODULE__{protocol: :udp}), do: 4

    defimpl Ockam.Router.Protocol.Encoder do
      def encode(value, opts),
        do: Ockam.Router.Protocol.Encoding.Default.Encoder.encode(value, opts)
    end

    defimpl Ockam.Router.Protocol.Encoding.Default.Encoder do
      alias Ockam.Router.Protocol.Encoding.Default.Encode
      alias Ockam.Router.Protocol.Endpoint.IPv4
      alias Ockam.Transport.Address

      def encode(%IPv4{address: address} = value, opts) do
        type = Encode.i1(IPv4.type_id(value), opts)
        addr = encode_address(address, opts)
        port = Encode.integer(Address.port(address), opts)
        {:ok, [type, addr, port]}
      end

      defp encode_address(address, _opts) do
        {a, b, c, d} = Address.ip(address)
        <<a::8, b::8, c::8, d::8>>
      end
    end

    defimpl Ockam.Router.Protocol.Decoder do
      def decode(value, input, opts),
        do: Ockam.Router.Protocol.Encoding.Default.Decoder.decode(value, input, opts)
    end

    defimpl Ockam.Router.Protocol.Encoding.Default.Decoder do
      alias Ockam.Router.Protocol.Encoding.Helpers
      alias Ockam.Router.Protocol.Endpoint.IPv4
      alias Ockam.Transport.Address

      def decode(value, <<addr::binary-size(4), rest::binary>>, %{protocol: protocol}) do
        {port, rest} = Helpers.decode_leb128_u2(rest)
        address = decode_address(addr, port)
        {:ok, %IPv4{value | protocol: protocol, address: address}, rest}
      end

      defp decode_address(<<a::8, b::8, c::8, d::8>>, port) do
        case {a, b, c, d} do
          {127, 0, 0, 1} ->
            %Address{family: :inet, addr: :loopback, port: port}

          {0, 0, 0, 0} ->
            %Address{family: :inet, addr: :any, port: port}

          addr ->
            %Address{family: :inet, addr: addr, port: port}
        end
      end
    end
  end

  defmodule IPv6 do
    defstruct [:protocol, :address]

    def type_id(%__MODULE__{protocol: :tcp}), do: 3
    def type_id(%__MODULE__{protocol: :udp}), do: 5

    defimpl Ockam.Router.Protocol.Encoder do
      def encode(value, opts),
        do: Ockam.Router.Protocol.Encoding.Default.Encoder.encode(value, opts)
    end

    defimpl Ockam.Router.Protocol.Encoding.Default.Encoder do
      alias Ockam.Router.Protocol.Encoding.Default.Encode
      alias Ockam.Router.Protocol.Endpoint.IPv6
      alias Ockam.Transport.Address

      def encode(%IPv6{address: address} = value, opts) do
        type = Encode.i1(IPv6.type_id(value), opts)
        addr = encode_address(address, opts)
        port = Encode.integer(Address.port(address), opts)
        {:ok, [type, addr, port]}
      end

      defp encode_address(address, _opts) do
        {a, b, c, d, e, f, g, h} = Address.ip(address)

        <<a::16, b::16, c::16, d::16, e::16, f::16, g::16, h::16>>
      end
    end

    defimpl Ockam.Router.Protocol.Decoder do
      def decode(value, input, opts),
        do: Ockam.Router.Protocol.Encoding.Default.Decoder.decode(value, input, opts)
    end

    defimpl Ockam.Router.Protocol.Encoding.Default.Decoder do
      alias Ockam.Router.Protocol.Encoding.Helpers
      alias Ockam.Router.Protocol.Endpoint.IPv6
      alias Ockam.Transport.Address

      def decode(value, <<addr::binary-size(16), rest::binary>>, %{protocol: protocol}) do
        {port, rest} = Helpers.decode_leb128_u2(rest)
        address = decode_address(addr, port)
        {:ok, %IPv6{value | protocol: protocol, address: address}, rest}
      end

      defp decode_address(<<a::16, b::16, c::16, d::16, e::16, f::16, g::16, h::16>>, port) do
        %Address{family: :inet, addr: {a, b, c, d, e, f, g, h}, port: port}
      end
    end
  end

  def new(endpoint) do
    %__MODULE__{value: endpoint}
  end

  def new_ipv4(:tcp, %Address{} = address) do
    %__MODULE__{value: %IPv4{protocol: :tcp, address: address}}
  end

  def new_ipv4(:udp, %Address{} = address) do
    %__MODULE__{value: %IPv4{protocol: :udp, address: address}}
  end

  def new_ipv6(:tcp, %Address{} = address) do
    %__MODULE__{value: %IPv6{protocol: :tcp, address: address}}
  end

  def new_ipv6(:udp, %Address{} = address) do
    %__MODULE__{value: %IPv6{protocol: :udp, address: address}}
  end

  def value(%__MODULE__{value: value}), do: value

  defimpl Ockam.Router.Protocol.Encoder do
    def encode(value, opts),
      do: Ockam.Router.Protocol.Encoding.Default.Encoder.encode(value, opts)
  end

  defimpl Ockam.Router.Protocol.Encoding.Default.Encoder do
    alias Ockam.Router.Protocol.Endpoint

    def encode(%Endpoint{value: value}, opts),
      do: Ockam.Router.Protocol.Encoding.Default.Encoder.encode(value, opts)
  end

  defimpl Ockam.Router.Protocol.Decoder do
    def decode(value, input, opts),
      do: Ockam.Router.Protocol.Encoding.Default.Decoder.decode(value, input, opts)
  end

  defimpl Ockam.Router.Protocol.Encoding.Default.Decoder do
    alias Ockam.Router.Protocol.DecodeError
    alias Ockam.Router.Protocol.Encoding.Default.Decoder
    alias Ockam.Router.Protocol.Endpoint

    def decode(value, <<type::8, input::binary>>, opts) do
      with {:ok, endpoint_mod, opts} <- endpoint_type(type, opts),
           {:ok, endpoint, rest} <- decode_endpoint(endpoint_mod, input, opts) do
        {:ok, %Endpoint{value | value: endpoint}, rest}
      end
    end

    defp decode_endpoint(mod, input, opts) do
      Decoder.decode(struct(mod, []), input, opts)
    end

    defp endpoint_type(0, opts), do: {:ok, Endpoint.Local, opts}
    defp endpoint_type(1, opts), do: {:ok, Endpoint.Channel, opts}
    defp endpoint_type(2, opts), do: {:ok, Endpoint.IPv4, Map.put(opts, :protocol, :tcp)}
    defp endpoint_type(3, opts), do: {:ok, Endpoint.IPv6, Map.put(opts, :protocol, :tcp)}
    defp endpoint_type(4, opts), do: {:ok, Endpoint.IPv4, Map.put(opts, :protocol, :udp)}
    defp endpoint_type(5, opts), do: {:ok, Endpoint.IPv6, Map.put(opts, :protocol, :udp)}
    defp endpoint_type(n, _opts), do: {:error, DecodeError.new({:invalid_endpoint_type, n})}
  end
end
