defmodule Ockam.Wire.Binary.V2 do
  @moduledoc false

  @behaviour Ockam.Wire

  alias Ockam.Message
  alias Ockam.Wire.Binary.V2.Route
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

  def encode(message) do
    onward_route = Message.onward_route(message)
    return_route = Message.return_route(message)
    payload = Message.payload(message)

    with {:ok, encoded_onward_route} <- Route.encode(onward_route),
         {:ok, encoded_return_route} <- Route.encode(return_route),
         encoded <-
           :bare.encode(
             %{
               version: @version,
               onward_route: encoded_onward_route,
               return_route: encoded_return_route,
               payload: payload
             },
             bare_spec(:message)
           ) do
      {:ok, encoded}
    end
  end

  @doc """
  Decodes a message from a binary.

  Returns `{:ok, message}`, if it succeeds.
  Returns `{:error, error}`, if it fails.
  """
  @spec decode(encoded :: binary()) ::
          {:ok, message :: Message.t()} | {:error, error :: DecodeError.t()}

  def decode(encoded) do
    with {:ok, %{onward_route: onward_route, return_route: return_route} = decoded, _} <-
           :bare.decode(encoded, bare_spec(:message)),
         {:ok, decoded_onward_route} <- Route.decode(onward_route),
         {:ok, decoded_return_route} <- Route.decode(return_route) do
      {:ok,
       Map.merge(decoded, %{
         onward_route: decoded_onward_route,
         return_route: decoded_return_route
       })}
    else
      foo -> {:error, foo}
    end
  end

  @doc """
  Formats an error returned by `Ockam.Wire.encode/1` or `Ockam.Wire.decode/1`.

  Returns a string.
  """
  @spec format_error(error :: EncodeError.t() | DecodeError.t()) ::
          formatted_error_message :: String.t()

  def format_error(error), do: "Unexpected error: #{inspect(error, as_binary: true)}"
end
