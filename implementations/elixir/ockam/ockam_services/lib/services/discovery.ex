defmodule Ockam.Services.Discovery do
  @moduledoc """
  Discovery service storing information about other services

  Options:
  storage: storage module to use, default is `Ockam.Services.Discovery.Storage.Memory`
  storage_options: options to call storage.init/1 with
  """

  use Ockam.Worker

  alias Ockam.Bare.Extended, as: BareExtended
  alias Ockam.Message
  alias Ockam.Router
  alias Ockam.Services.Discovery.ServiceInfo

  require Logger

  @impl true
  def setup(options, state) do
    storage = Keyword.get(options, :storage, Ockam.Services.Discovery.Storage.Memory)
    storage_options = Keyword.get(options, :storage_options, [])

    {:ok, Map.put(state, :storage, {storage, storage.init(storage_options)})}
  end

  @impl true
  def handle_message(message, state) do
    result =
      case parse_request(message) do
        :list ->
          list(state)

        {:get, id} ->
          get(id, state)

        {:register, id, route, metadata} ->
          ## Don't reply to register request
          ## TODO: register API with replies
          case register(id, route, metadata, state) do
            {:ok, state} ->
              {:noreply, state}

            other ->
              other
          end

        other ->
          Logger.warn(
            "Unable to parse message payload: #{inspect(message)} reason: #{inspect(other)}"
          )

          {:noreply, state}
      end

    reply(result, message)
  end

  def with_storage(state, fun) do
    {storage_mod, storage_state} = Map.get(state, :storage)
    {result, new_storage_state} = fun.(storage_mod, storage_state)
    {result, Map.put(state, :storage, {storage_mod, new_storage_state})}
  end

  def list(state) do
    with_storage(state, fn storage_mod, storage_state ->
      storage_mod.list(storage_state)
    end)
  end

  def get(id, state) do
    with_storage(state, fn storage_mod, storage_state ->
      storage_mod.get(id, storage_state)
    end)
  end

  def register(id, route, metadata, state) do
    with_storage(state, fn storage_mod, storage_state ->
      storage_mod.register(id, route, metadata, storage_state)
    end)
  end

  def parse_request(message) do
    payload = Message.payload(message)

    case payload do
      <<0>> <> request_v0 ->
        ## TODO: better way to encode request data??
        case BareExtended.decode(request_v0, request_schema()) do
          {:ok, {:list, ""}} ->
            :list

          {:ok, {:get, id}} ->
            {:get, id}

          {:ok, {:register, %{id: id, metadata: metadata}}} ->
            ## Using message return route as a route in register request.
            {:register, id, Message.return_route(message), metadata}

          other ->
            other
        end

      other ->
        {:error, {:invalid_request_version, other}}
    end
  end

  def reply({:noreply, state}, _message) do
    {:ok, state}
  end

  def reply({reply, state}, message) do
    Router.route(Message.reply(message, state.address, format_reply(reply)))
    {:ok, state}
  end

  def format_reply(reply) do
    ## TODO: maybe use better distinction between results (request id/function?)
    formatted =
      case reply do
        {:ok, service_info} ->
          encode_service_info(service_info)

        [] ->
          encode_service_infos([])

        [%ServiceInfo{} | _] = list ->
          encode_service_infos(list)

        :ok ->
          ## TODO: meaningful response for registration
          ""

        {:error, _reason} ->
          ## TODO: error encoding
          ""
      end

    <<0>> <> formatted
  end

  ## BARE schemas

  def request_schema() do
    [
      list: {:data, 0},
      get: :string,
      register: {:struct, [id: :string, metadata: {:map, :string, :data}]}
    ]
  end

  ## To be used with this schema, routes should be normalized to (type, value) maps
  ## TODO: improve encode/decode logic to work with other address formats
  def service_info_schema() do
    {:struct,
     [
       id: :string,
       route: Ockam.Wire.Binary.V2.bare_spec(:route),
       metadata: {:map, :string, :data}
     ]}
  end

  ## TODO: come up with better API for encoding/decoding of routes
  def encode_service_info(service_info) do
    service_info = normalize_service_info(service_info)
    :bare.encode(service_info, service_info_schema())
  end

  def encode_service_infos(service_infos) do
    service_infos =
      Enum.map(service_infos, fn service_info -> normalize_service_info(service_info) end)

    :bare.encode(service_infos, {:array, service_info_schema()})
  end

  def normalize_service_info(%{route: route} = service_info) do
    normalized_route = Enum.map(route, fn address -> Ockam.Address.normalize(address) end)
    Map.put(service_info, :route, normalized_route)
  end
end
