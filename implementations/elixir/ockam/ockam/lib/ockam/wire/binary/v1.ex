defmodule Ockam.Wire.Binary.V1 do
  @moduledoc false

  @behaviour Ockam.Wire

  alias Ockam.Address
  alias Ockam.Message

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
          {:ok, encoded :: iodata}

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
          {:ok, message :: Message.t()} | {:error, error :: any()}

  def decode(encoded) do
    ## Expect first byte to be the version
    case encoded do
      <<@version, _rest::binary>> ->
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

          {:ok, _decoded, rest} ->
            {:error, {:too_much_data, encoded, rest}}

          {:error, reason} ->
            {:error, reason}
        end

      <<wrong_version, _rest::binary>> ->
        {:error, {:invalid_version, encoded, wrong_version}}
    end
  end

  def encode_route(route) do
    {:ok, :bare.encode(normalize_route(route), bare_spec(:route))}
  end

  def decode_route(encoded_route) do
    case :bare.decode(encoded_route, bare_spec(:route)) do
      {:ok, route, ""} ->
        {:ok, denormalize_route(route)}

      {:ok, _decoded, rest} ->
        {:error, {:too_much_data, encoded_route, rest}}

      {:error, reason} ->
        {:error, reason}
    end
  end

  def encode_address(address) do
    {:ok, :bare.encode(Address.normalize(address), bare_spec(:address))}
  end

  def decode_address(encoded_address) do
    case :bare.decode(encoded_address, bare_spec(:address)) do
      {:ok, address, ""} ->
        {:ok, Address.denormalize(address)}

      {:ok, _decoded, rest} ->
        {:error, {:too_much_data, encoded_address, rest}}

      {:error, reason} ->
        {:error, reason}
    end
  end

  def normalize_route(route) when is_list(route) do
    ## TODO: check if all addresses are valid
    Enum.map(route, &Address.normalize/1)
  end

  def denormalize_route(addresses) when is_list(addresses) do
    Enum.map(addresses, &Address.denormalize/1)
  end
end
