defmodule Ockam.API.Discovery.ServiceInfo do
  @moduledoc """
  Service info structure for discovery service.
  """

  defstruct [:id, :route, metadata: %{}]

  @type t() :: %__MODULE__{
          id: binary(),
          route: [Ockam.Address.t()],
          metadata: %{binary() => binary()}
        }

  defmodule NormalizedServiceInfo do
    @moduledoc """
    ServiceInfo, normalized, to encode/decode from cbor
    Note the Address is not encoded "minified" but rather
    it is encoded as a raw maps. This probably should be improved
    to avoid encoding "type" and "value" on each one, plus helping
    on ensure their correct typing.
    """
    use TypedStruct

    typedstruct do
      plugin(Ockam.TypedCBOR.Plugin)
      field(:id, binary(), minicbor: [key: 1])
      field(:route, list(map()), minicbor: [key: 2])
      field(:metadata, %{binary() => binary()}, minicbor: [key: 3])
    end
  end

  def encode_list!(service_infos) do
    service_infos |> Enum.map(&normalize/1) |> NormalizedServiceInfo.encode_list!()
  end

  def decode_list(data) do
    case NormalizedServiceInfo.decode_list(data) do
      {:ok, list, ""} ->
        Enum.map(list, &denormalize/1)

      ## TODO: move that to mini_cbor
      {:ok, map, rest} ->
        {:error, {:decode_error, {:extra_data, rest, map}, data}}

      {:error, reason} ->
        {:error, reason}
    end
  end

  def encode!(%__MODULE__{} = service_info) do
    normalize(service_info) |> NormalizedServiceInfo.encode!()
  end

  def decode(data) do
    case NormalizedServiceInfo.decode(data) do
      {:ok, normalized, ""} ->
        {:ok, denormalize(normalized)}

      ## TODO: move that to mini_cbor
      {:ok, map, rest} ->
        {:error, {:decode_error, {:extra_data, rest, map}, data}}

      {:error, reason} ->
        {:error, reason}
    end
  end

  defp normalize(%__MODULE__{id: id, route: route, metadata: metadata}) do
    normalized_route = Enum.map(route, fn address -> Ockam.Address.normalize(address) end)
    %NormalizedServiceInfo{id: id, route: normalized_route, metadata: metadata}
  end

  defp denormalize(%NormalizedServiceInfo{id: id, route: route, metadata: metadata}) do
    denormalized_route = Enum.map(route, fn address -> Ockam.Address.denormalize(address) end)
    %__MODULE__{id: id, route: denormalized_route, metadata: metadata}
  end
end
