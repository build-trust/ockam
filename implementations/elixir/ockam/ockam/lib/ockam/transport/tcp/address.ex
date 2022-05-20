defmodule Ockam.Transport.TCPAddress do
  @moduledoc """
  Functions to work with TCP transport address
  """
  alias Ockam.Address

  @address_type 1

  @type t :: Address.t(1)

  def type(), do: @address_type

  @spec to_host_port(t()) :: {:ok, {String.t(), integer()}} | {:error, any()}
  def to_host_port(address) do
    case is_tcp_address(address) do
      true ->
        parse_host_port(Address.value(address))

      false ->
        {:error, {:invalid_address_type, Address.type(address)}}
    end
  end

  def parse_host_port(value) do
    case Regex.split(~r/(:)\d*$/, value, include_captures: true, on: :all_but_first, trim: true) do
      [host, ":", port_str] ->
        case Integer.parse(port_str) do
          {port, ""} ->
            {:ok, {host, port}}

          _other ->
            {:error, {:invalid_port, port_str}}
        end

      _other ->
        {:error, {:invalid_host_port, value}}
    end
  end

  @spec new(String.t() | :inet.ip_address(), integer()) :: t()
  def new(host, port) when (is_binary(host) or is_tuple(host)) and is_integer(port) do
    %Address{type: @address_type, value: format_host_port(host, port)}
  end

  @spec new(String.t()) :: t()
  def new(host_port) when is_binary(host_port) or is_list(host_port) do
    case parse_host_port(host_port) do
      {:ok, {host, port}} -> new(host, port)
      error -> raise "Invalid host port string: #{inspect(error)}"
    end
  end

  def format_host_port(host, port) when is_binary(host) do
    "#{host}:#{port}"
  end

  def format_host_port(ip, port) when is_tuple(ip) do
    ## TODO: format IPV6 with brackets
    host = to_string(:inet.ntoa(ip))
    format_host_port(host, port)
  end

  def is_tcp_address(address) do
    Address.type(address) == @address_type
  end
end
