defmodule Ockam.Wire.Binary.V2.Route do
  @moduledoc false

  alias Ockam.Address
  alias Ockam.Wire.DecodeError
  alias Ockam.Wire.EncodeError

  require DecodeError
  require EncodeError

  @type formatted_address() :: %{type: Address.type(), value: binary()}

  @spec encode(any) :: {:ok, list(formatted_address)}
  def encode(route) when is_list(route) do
    {:ok, Enum.map(route, &encode_address/1)}
  end

  def encode(input) do
    reason = {:argument_is_not_a_route, input}
    {:error, EncodeError.new(reason)}
  end

  @spec decode(maybe_improper_list) :: {:error, DecodeError.t()} | {:ok, list}
  @doc """
  Decodes a route from a binary.

  Returns {:ok, routes} if it succeeds.
  Returns {:error, successful_routes} if it fails.
  """
  def decode(addresses) when is_list(addresses) and length(addresses) > 0 do
    # TODO: this is also kinda ugly
    decoded =
      Enum.map(addresses, fn address ->
        decode_address(address)
      end)

    if length(decoded) == length(addresses) do
      {:ok, decoded}
    else
      # should return an actual error instead of only successful routes.
      r = {:an_address_failed_to_encode, [decoded: decoded, input: addresses]}
      {:error, DecodeError.new(r)}
    end
  end

  def decode([]), do: {:ok, []}

  @spec encode_address(Address.t()) :: formatted_address()
  def encode_address(address) do
    %{type: Address.type(address), value: Address.value(address)}
  end

  @spec decode_address(formatted_address()) :: Address.t()
  def decode_address(%{type: type, value: value}) do
    # TODO: there needs to be a way to do this programmatically
    case type do
      0 ->
        value

      _other ->
        %Address{type: type, value: value}
    end
  end
end
