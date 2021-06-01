defmodule Ockam.Wire.Binary.V2 do
  @moduledoc false

  @behaviour Ockam.Wire

  alias Ockam.Address
  alias Ockam.Message
  alias Ockam.Wire.DecodeError
  alias Ockam.Wire.EncodeError

  require DecodeError
  require EncodeError

  @version 1

  # TODO: refactor this.
  def bare_spec(:address) do
    {:struct, [type: :uint, value: :data]}
  end

  def bare_spec(:route) do
    {:array, bare_spec(:address)}
  end

  def bare_spec(:message) do
    {:struct,
     [
       version: :uint,
       onward_route: bare_spec(:route),
       return_route: bare_spec(:route),
       payload: :data
     ]}
  end

  @doc """
  Encodes a message into a binary.

  Returns `{:ok, iodata}`, if it succeeds.
  Returns `{:error, error}`, if it fails.
  """
  @spec encode(message :: Message.t()) ::
          {:ok, encoded :: iodata} | {:error, error :: EncodeError.t()}

  def encode(%Ockam.Message{} = message) do
    onward_route = Message.onward_route(message)
    return_route = Message.return_route(message)
    payload = Message.payload(message)

    ## TODO: validate data and handle errors?
    encoded =
      :bare.encode(
        %{
          version: @version,
          onward_route: normalize_route(onward_route),
          return_route: normalize_route(return_route),
          payload: payload
        },
        bare_spec(:message)
      )

    {:ok, encoded}
  end

  @doc """
  Decodes a message from a binary.

  Returns `{:ok, message}`, if it succeeds.
  Returns `{:error, error}`, if it fails.
  """
  @spec decode(encoded :: binary()) ::
          {:ok, message :: Message.t()} | {:error, error :: DecodeError.t()}

  def decode(encoded) do
    case :bare.decode(encoded, bare_spec(:message)) do
      {:ok, %{onward_route: onward_route, return_route: return_route} = decoded, ""} ->
        {:ok,
         struct(
           Ockam.Message,
           Map.merge(decoded, %{
             onward_route: denormalize_route(onward_route),
             return_route: denormalize_route(return_route)
           })
         )}

      other ->
        {:error, DecodeError.new(other)}
    end
  end

  @doc """
  Formats an error returned by `Ockam.Wire.encode/1` or `Ockam.Wire.decode/1`.

  Returns a string.
  """
  @spec format_error(error :: EncodeError.t() | DecodeError.t()) ::
          formatted_error_message :: String.t()

  def format_error(error), do: "Unexpected error: #{inspect(error, as_binary: true)}"

  def normalize_route(route) when is_list(route) do
    ## TODO: check if all addresses are valid
    Enum.map(route, &Address.normalize/1)
  end

  def denormalize_route(addresses) when is_list(addresses) do
    Enum.map(addresses, &Address.denormalize/1)
  end
end
