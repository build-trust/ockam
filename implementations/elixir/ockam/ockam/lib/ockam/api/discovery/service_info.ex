defmodule Ockam.API.Discovery.ServiceInfo do
  @moduledoc """
  Service info structure for discovery service.
  """
  defstruct [:id, :route, metadata: %{}]

  @schema {:map, [:id, :route, :metadata]}
  @list_schema {:list, @schema}

  @type t() :: %__MODULE__{
          id: binary(),
          route: [Ockam.Address.t()],
          metadata: %{binary() => binary()}
        }

  def encode_list(service_infos) do
    Enum.map(service_infos, &normalize/1)
    |> MiniCBOR.encode(@list_schema)
  end

  def decode_list(data) do
    case MiniCBOR.decode(data, @list_schema) do
      {:ok, list, ""} ->
        Enum.map(list, &denormalize/1)

      ## TODO: move that to mini_cbor
      {:ok, map, rest} ->
        {:error, {:decode_error, {:extra_data, rest, map}, data}}

      {:error, reason} ->
        {:error, reason}
    end
  end

  ## TODO: try defimpl CBOR.Encoder, for: __MODULE__
  def encode(%__MODULE__{} = service_info) do
    service_info = normalize(service_info)
    MiniCBOR.encode(service_info, @schema)
  end

  def decode(data) do
    case MiniCBOR.decode(data, @schema) do
      {:ok, map, ""} ->
        {:ok, denormalize(map)}

      ## TODO: move that to mini_cbor
      {:ok, map, rest} ->
        {:error, {:decode_error, {:extra_data, rest, map}, data}}

      {:error, reason} ->
        {:error, reason}
    end
  end

  def normalize(%__MODULE__{route: route} = service_info) do
    normalized_route = Enum.map(route, fn address -> Ockam.Address.normalize(address) end)
    Map.put(service_info, :route, normalized_route)
  end

  def denormalize(%{route: route} = service_info) do
    denormalized_route = Enum.map(route, fn address -> Ockam.Address.denormalize(address) end)
    struct(__MODULE__, Map.put(service_info, :route, denormalized_route))
  end
end
