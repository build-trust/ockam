defmodule Ockam.Transport.UDPAddress do
  @moduledoc false

  alias __MODULE__

  defstruct [:ip, :port]

  # udp address type
  @udp 2

  # ip address type tags
  @ipv4 0
  @ipv6 1

  def deserialize(value) when is_list(value), do: deserialize(IO.iodata_to_binary(value))

  def deserialize(
        <<@udp::8, 7::8, @ipv4::8, a::8, b::8, c::8, d::8, port::unsigned-little-integer-16>>
      ) do
    %UDPAddress{ip: {a, b, c, d}, port: port}
  end

  def deserialize(
        <<@udp::8, 17::8, @ipv6::8, a::8, b::8, c::8, d::8, e::8, f::8, g::8, h::8,
          port::unsigned-little-integer-16>>
      ) do
    %UDPAddress{ip: {a, b, c, d, e, f, g, h}, port: port}
  end

  def deserialize(value), do: {:error, {:not_a_valid_serialized_udp_address, value}}
end

defimpl Ockam.Address, for: Ockam.Transport.UDPAddress do
  alias Ockam.Transport.UDPAddress

  def type(_address), do: 2
  def value(address), do: address
end

defimpl Ockam.Serializable, for: Ockam.Transport.UDPAddress do
  alias Ockam.Transport.UDPAddress

  # udp address type
  @udp 2

  # ip address type tags
  @ipv4 0
  @ipv6 1

  def serialize(%UDPAddress{ip: ip, port: port}) do
    with {:ok, serialized_ip} <- serialize_ip(ip),
         {:ok, serialized_port} <- serialize_port(port) do
      length = byte_size(serialized_ip) + byte_size(serialized_port)
      %{type: 2, value: :binary.list_to_bin([@udp, <<length::8>>, serialized_ip, serialized_port])}
    end
  end

  # Turns and IP into a binary.

  defp serialize_ip({a, b, c, d}) do
    {:ok, <<@ipv4::8, a::8, b::8, c::8, d::8>>}
  end

  defp serialize_ip({a, b, c, d, e, f, g, h}) do
    {:ok,
     <<@ipv6::8, a::unsigned-little-integer-16, b::unsigned-little-integer-16,
       c::unsigned-little-integer-16, d::unsigned-little-integer-16,
       e::unsigned-little-integer-16, f::unsigned-little-integer-16,
       g::unsigned-little-integer-16, h::unsigned-little-integer-16>>}
  end

  defp serialize_ip(value), do: {:error, {:not_a_valid_ip, value}}

  # Turn a ports into a binary.

  defp serialize_port(port) when port >= 0 and port <= 65_535 do
    {:ok, <<port::unsigned-little-integer-16>>}
  end

  defp serialize_port(value), do: {:error, {:not_a_valid_port, value}}
end
