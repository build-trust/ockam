defmodule Ockam.Transport.UDPAddress do
  @moduledoc """
  Functions to work with UDP transport address
  """

  alias Ockam.Address

  # udp address type
  @address_type 2

  def type(), do: @address_type

  def new(ip, port) do
    value = format_ip_port(ip, port)

    %Address{type: @address_type, value: value}
  end

  def is_udp_address(address) do
    Address.type(address) == @address_type
  end

  def to_ip_port(address) do
    case is_udp_address(address) do
      true ->
        parse_ip_port(Address.value(address))

      false ->
        {:error, {:invalid_address_type, Address.type(address)}}
    end
  end

  def format_ip_port(ip, port) when is_tuple(ip) do
    ip_str = to_string(:inet.ntoa(ip))
    "#{ip_str}:#{port}"
  end

  def format_ip_port(ip_str, port) when is_binary(ip_str) do
    {:ok, _ip} = :inet.parse_address(to_charlist(ip_str))
    "#{ip_str}:#{port}"
  end

  defp parse_ip_port(value) do
    case Regex.split(~r/(:)\d*$/, value, include_captures: true, on: :all_but_first, trim: true) do
      [ip_str, ":", port_str] ->
        with {:ok, port} <- parse_port(port_str),
             {:ok, ip} <- parse_ip(ip_str) do
          {:ok, {ip, port}}
        else
          error -> error
        end

      _other ->
        {:error, {:invalid_host_port, value}}
    end
  end

  defp parse_ip(ip_str) do
    case :inet.parse_address(to_charlist(ip_str)) do
      {:ok, ip} -> {:ok, ip}
      __other -> {:error, {:invalid_ip_address, ip_str}}
    end
  end

  defp parse_port(port_str) do
    case Integer.parse(port_str) do
      {port, ""} ->
        {:ok, port}

      _other ->
        {:error, {:invalid_port, port_str}}
    end
  end
end
