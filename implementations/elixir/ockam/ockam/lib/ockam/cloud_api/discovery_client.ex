defmodule Ockam.CloudApi.DiscoveryClient do
  @moduledoc """
  Client API for discovery service
  """

  alias Ockam.Bare.Extended, as: BareExtended
  alias Ockam.Router

  require Logger

  def register_service(discovery_route, id, route, metadata \\ %{}) do
    Logger.info("Registering #{inspect(id)}, #{inspect(route)} #{inspect(metadata)}")
    payload = encode_register_request(id, metadata)
    Router.route(payload, discovery_route, route)
  end

  def list_services(access_route, discovery_route) do
    discovery_service_route = access_route ++ discovery_route
    payload = encode_list_request()

    case Ockam.Workers.Call.call_on_current_process(payload, discovery_service_route) do
      {:ok, %Ockam.Message{payload: list_response}} ->
        case parse_list_response(list_response) do
          {:ok, service_infos} ->
            {:ok, extend_routes_with_access_route(service_infos, access_route)}

          {:error, reason} ->
            {:error, reason}
        end

      other ->
        other
    end
  end

  def extend_routes_with_access_route(service_infos, access_route) do
    Enum.map(service_infos, fn %{route: route} = service_info ->
      Map.put(service_info, :relative_route, access_route ++ route)
    end)
  end

  def denormalize_routes(service_infos) do
    Enum.map(service_infos, fn service_info ->
      Map.update!(service_info, :route, fn route ->
        denormalize_route(route)
      end)
    end)
  end

  defp denormalize_route(route) do
    Enum.map(route, fn address -> Ockam.Address.denormalize(address) end)
  end

  def parse_list_response(list_response) do
    <<0>> <> list = list_response

    case :bare.decode(list, {:array, service_info_schema()}) do
      {:ok, infos, ""} -> {:ok, denormalize_routes(infos)}
      {:ok, _infos, _rest} = result -> {:error, {:cannot_decode, list_response, result}}
      {:error, reason} -> {:error, reason}
    end
  end

  ## TODO: share the schema with the service
  def service_info_schema() do
    {:struct,
     [
       id: :string,
       route: Ockam.Wire.Binary.V2.bare_spec(:route),
       metadata: {:map, :string, :data}
     ]}
  end

  def request_schema() do
    [
      list: {:data, 0},
      get: :string,
      register: {:struct, [id: :string, metadata: {:map, :string, :data}]}
    ]
  end

  def encode_list_request() do
    <<0>> <> BareExtended.encode({:list, ""}, request_schema())
  end

  def encode_register_request(id, metadata) do
    <<0>> <> BareExtended.encode({:register, %{id: id, metadata: metadata}}, request_schema())
  end
end
