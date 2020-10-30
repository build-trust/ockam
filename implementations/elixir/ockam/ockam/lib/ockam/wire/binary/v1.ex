defmodule Ockam.Wire.Binary.V1 do
  @moduledoc false

  @behaviour Ockam.Wire

  alias Ockam.Message
  alias Ockam.Serializable
  alias Ockam.Wire.Binary.V1.Route
  alias Ockam.Wire.Binary.VarInt
  alias Ockam.Wire.DecodeError
  alias Ockam.Wire.EncodeError

  require DecodeError
  require EncodeError

  @version 1

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

    with {:ok, encoded_version} <- encode_version(),
         {:ok, encoded_onward_route} <- Route.encode(onward_route),
         {:ok, encoded_return_route} <- Route.encode(return_route),
         {:ok, encoded_payload} <- encode_payload(payload) do
      {:ok, [encoded_version, encoded_onward_route, encoded_return_route, encoded_payload]}
    end
  end

  def encode_version do
    case VarInt.encode(@version) do
      {:error, error} -> {:error, error}
      encoded -> {:ok, encoded}
    end
  end

  def encode_payload(payload) do
    case Serializable.impl_for(payload) do
      nil ->
        {:error, EncodeError.new({:payload_is_not_serializable, payload})}

      _impl ->
        case Serializable.serialize(payload) do
          {:error, reason} -> {:error, EncodeError.new(reason)}
          serialized -> {:ok, serialized}
        end
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
    with {:ok, @version, rest} <- decode_version(encoded),
         {:ok, onward_route, rest} <- Route.decode(rest),
         {:ok, return_route, rest} <- Route.decode(rest) do
      {:ok, %{onward_route: onward_route, return_route: return_route, payload: rest}}
    end
  end

  defp decode_version(encoded) do
    case VarInt.decode(encoded) do
      {:error, error} ->
        {:error, error}

      {@version, rest} ->
        {:ok, @version, rest}

      {v, rest} ->
        r = {:unexpected_version, [expected: @version, decoded: v, input: encoded, rest: rest]}
        {:error, DecodeError.new(r)}
    end
  end

  @doc """
  Formats an error returned by `Ockam.Wire.encode/1` or `Ockam.Wire.decode/1`.

  Returns a string.
  """
  @spec format_error(error :: EncodeError.t() | DecodeError.t()) ::
          formatted_error_message :: String.t()

  def format_error(error), do: "Unexpected error: #{inspect(error)}"
end
