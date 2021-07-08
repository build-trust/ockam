defmodule Ockam.Transport.TCPAddress do
  @moduledoc false

  alias __MODULE__

  alias Ockam.Bare.Extended, as: BareExtended

  defstruct [:host, :port]

  @ipv4_type {:data, 4}
  @ipv6_type {:data, 16}
  @host_type :string
  ## type TCPAddress {
  ##   host: ({:data, 4} | {:data, 16} | string) // host, ipv4 or ipv6
  ##   port: u16
  ## }
  def bare_schema() do
    {:struct, [host: {:union, [@ipv4_type, @ipv6_type, @host_type]}, port: :u16]}
  end

  def decode(value) when is_binary(value) do
    case BareExtended.decode(value, bare_schema()) do
      {:ok, %{port: port, host: decoded_host}} ->
        host = parse_host(decoded_host)

        %TCPAddress{host: host, port: port}

      {:error, reason} ->
        {:error, {:not_a_valid_serialized_tcp_address, reason}}
    end
  end

  def encode(%TCPAddress{host: host, port: port}) do
    BareExtended.encode(%{port: port, host: format_host(host)}, bare_schema())
  end

  def format_host({a, b, c, d}) do
    {@ipv4_type, <<a::8, b::8, c::8, d::8>>}
  end

  def format_host({a, b, c, d, e, f, g, h}) do
    {@ipv6_type,
     <<a::unsigned-little-integer-16, b::unsigned-little-integer-16,
       c::unsigned-little-integer-16, d::unsigned-little-integer-16,
       e::unsigned-little-integer-16, f::unsigned-little-integer-16,
       g::unsigned-little-integer-16, h::unsigned-little-integer-16>>}
  end

  def format_host(string) when is_binary(string) do
    {@host_type, string}
  end

  def parse_host({@host_type, host}) do
    host
  end

  def parse_host({@ipv4_type, <<a::8, b::8, c::8, d::8>>}) do
    {a, b, c, d}
  end

  def parse_host(
        {@ipv6_type,
         <<a::unsigned-little-integer-16, b::unsigned-little-integer-16,
           c::unsigned-little-integer-16, d::unsigned-little-integer-16,
           e::unsigned-little-integer-16, f::unsigned-little-integer-16,
           g::unsigned-little-integer-16, h::unsigned-little-integer-16>>}
      ) do
    {a, b, c, d, e, f, g, h}
  end

  # tcp address type
  @tcp 1

  @spec deserialize(any) ::
          {:error, {:not_a_valid_serialized_tcp_address, any}}
          | %Ockam.Transport.TCPAddress{
              host: {byte, byte, byte, byte} | {byte, byte, byte, byte, byte, byte, byte, byte},
              port: char
            }

  def deserialize(%{type: @tcp, value: value}), do: deserialize(value)

  def deserialize(value) when is_list(value), do: deserialize(IO.iodata_to_binary(value))

  def deserialize(data) do
    decode(data)
  end
end

defimpl Ockam.Address, for: Ockam.Transport.TCPAddress do
  alias Ockam.Transport.TCPAddress

  def type(_address), do: 1
  def value(address), do: address
end

defimpl Ockam.Serializable, for: Ockam.Transport.TCPAddress do
  alias Ockam.Transport.TCPAddress

  # tcp address type
  @tcp 1

  def serialize(%TCPAddress{} = tcp_address) do
    %{type: @tcp, value: Ockam.Transport.TCPAddress.encode(tcp_address)}
  end
end
