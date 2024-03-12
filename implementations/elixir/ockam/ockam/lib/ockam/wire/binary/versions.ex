defmodule Ockam.Wire.Binary.Versions do
  @moduledoc false

  @behaviour Ockam.Wire

  alias Ockam.Address
  alias Ockam.Message

  @latest_version 2
  @version_1 1

  @address_spec {:struct, [type: :uint, value: :data]}
  @route_spec {:array, @address_spec}
  @message_fields [
    version: :uint,
    onward_route: @route_spec,
    return_route: @route_spec,
    payload: :data
  ]
  @message_spec {:struct, @message_fields}
  @message_with_tracing_context_spec {
    :struct,
    @message_fields ++ [tracing_context: {:optional, :string}]
  }

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

    ## TODO: validate data and handle errors
    encoded =
      Message.tracing_context(message)
      |> case do
        nil ->
          :bare.encode(
            %{
              version: @version_1,
              onward_route: normalize_route(onward_route),
              return_route: normalize_route(return_route),
              payload: payload
            },
            @message_spec
          )

        context ->
          :bare.encode(
            %{
              version: @latest_version,
              onward_route: normalize_route(onward_route),
              return_route: normalize_route(return_route),
              payload: payload,
              tracing_context: context
            },
            @message_with_tracing_context_spec
          )
      end

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
      <<@version_1, _rest::binary>> ->
        case :bare.decode(encoded, @message_spec) do
          {:ok, %{onward_route: onward_route, return_route: return_route} = decoded, ""} ->
            {:ok,
             struct(
               Ockam.Message,
               Map.merge(decoded, %{
                 onward_route: denormalize_route(onward_route),
                 return_route: denormalize_route(return_route)
               })
             )}

          {:error, reason} ->
            {:error, reason}
        end

      <<@latest_version, _rest::binary>> ->
        case :bare.decode(encoded, @message_spec) do
          {:ok, _decoded, _rest} ->
            decode_with_tracing_context(encoded)

          {:error, reason} ->
            {:error, reason}
        end

      <<wrong_version, _rest::binary>> ->
        {:error, {:invalid_version, encoded, wrong_version}}
    end
  end

  def decode_with_tracing_context(encoded) do
    case :bare.decode(encoded, @message_with_tracing_context_spec) do
      {:ok,
       %{onward_route: onward_route, return_route: return_route, tracing_context: context} =
           decoded, ""} ->
        decoded = Map.delete(decoded, :tracing_context)

        {:ok,
         struct(
           Ockam.Message,
           Map.merge(decoded, %{
             onward_route: denormalize_route(onward_route),
             return_route: denormalize_route(return_route),
             local_metadata: %{tracing_context: context}
           })
         )}

      {:ok, _decoded, rest} ->
        {:error, {:too_much_data, encoded, rest}}

      {:error, reason} ->
        {:error, reason}
    end
  end

  def encode_route(route) do
    {:ok, :bare.encode(normalize_route(route), @route_spec)}
  end

  def decode_route(encoded_route) do
    case :bare.decode(encoded_route, @route_spec) do
      {:ok, route, ""} ->
        {:ok, denormalize_route(route)}

      {:ok, _decoded, rest} ->
        {:error, {:too_much_data, encoded_route, rest}}

      {:error, reason} ->
        {:error, reason}
    end
  end

  def encode_address(address) do
    {:ok, :bare.encode(Address.normalize(address), @address_spec)}
  end

  def decode_address(encoded_address) do
    case :bare.decode(encoded_address, @address_spec) do
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
