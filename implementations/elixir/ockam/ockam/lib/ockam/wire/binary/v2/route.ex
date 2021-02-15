defmodule Ockam.Wire.Binary.V2.Route do
  @moduledoc false

  alias Ockam.Wire.Binary.V1.Address
  alias Ockam.Wire.DecodeError
  alias Ockam.Wire.EncodeError

  require DecodeError
  require EncodeError

  def encode(route) when is_list(route) do
    case encode_addresses(route, []) do
      {:error, error} ->
        {:error, error}

      encoded_addresses ->
        {:ok, encoded_addresses}
    end
  end

  def encode(input) do
    reason = {:argument_is_not_a_route, input}
    {:error, EncodeError.new(reason)}
  end

  def encode_addresses([], encoded), do: Enum.reverse(encoded)

  def encode_addresses([address | remaining_route], encoded) do
    case Address.encode(address) do
      {:error, error} -> {:error, error}
      encoded_address -> encode_addresses(remaining_route, [encoded_address | encoded])
    end
  end

  @doc """
  Decodes a route from a binary.

  Returns `{:ok, route}`, if it succeeds.
  Returns `{:error, error}`, if it fails.
  """

  @spec decode(encoded :: binary()) ::
          {:ok, route :: [Ockam.Address.t()], rest :: binary()}
          | {:error, error :: DecodeError.t()}

  def decode(<<number_of_addresses::unsigned-integer-8, rest::binary>>) do
    case decode_addressses(number_of_addresses, [], rest) do
      {:error, error} -> {:error, error}
      {route, rest} -> {:ok, route, rest}
    end
  end

  def decode(encoded) do
    {:error, DecodeError.new({:could_not_decode_route, encoded})}
  end

  # recurse from n, n-1 .. 0 to decode a list of addresses
  # stop of Address.decode/1 returns an error or when n=0

  defp decode_addressses(0, addresses, rest), do: {Enum.reverse(addresses), rest}

  defp decode_addressses(n, addresses, encoded) do
    case Address.decode(encoded) do
      {:error, error} -> {:error, error}
      {address, rest} -> decode_addressses(n - 1, [address | addresses], rest)
    end
  end
end
