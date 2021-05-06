defmodule Ockam.Stream.Workers.Service do
  @moduledoc false

  use Ockam.Worker
  use Ockam.Protocol.Mapping

  alias Ockam.Stream.Workers.Stream, as: StreamWorker

  alias Ockam.Message

  alias Ockam.Protocol.Stream, as: StreamProtocol

  require Logger

  @type state() :: map()
  @type stream_id() :: {String.t(), integer()}

  @default_storage Ockam.Stream.Storage.Internal

  @protocol_mapping Ockam.Protocol.Mapping.mapping([
                      {:server, StreamProtocol.Create},
                      {:server, StreamProtocol.Partitioned.Create},
                      {:server, Ockam.Protocol.Error}
                    ])

  @impl true
  def protocol_mapping() do
    @protocol_mapping
  end

  @impl true
  def setup(options, state) do
    stream_options = stream_options(options)
    {:ok, Map.put(state, :stream_options, stream_options)}
  end

  @impl true
  def handle_message(%{payload: payload} = message, state) do
    state =
      case decode_payload(payload) do
        {:ok, StreamProtocol.Create, %{stream_name: name}} ->
          name = ensure_stream_name(name, state)

          ensure_streams(name, 1, message, state, protocol: StreamProtocol.Create)

        {:ok, StreamProtocol.Partitioned.Create, %{stream_name: name, partitions: n_partitions}} ->
          name = ensure_stream_name(name, state)

          ensure_streams(name, n_partitions, message, state,
            protocol: StreamProtocol.Partitioned.Create
          )

        {:error, error} ->
          return_error(error, message, state)
      end

    {:ok, state}
  end

  def ensure_stream_name(name, state) do
    case name do
      :undefined ->
        create_stream_name(state)

      name ->
        name
    end
  end

  def return_error(error, message, state) do
    Logger.error("Error creating stream: #{inspect(error)}")

    Ockam.Router.route(%{
      onward_route: Message.return_route(message),
      return_route: [state.address],
      payload: encode_payload(Ockam.Protocol.Error, %{reason: "Invalid request"})
    })
  end

  @spec ensure_streams(String.t(), integer(), map(), state(), Keyword.t()) :: state()
  def ensure_streams(name, n_partitions, message, state, options) do
    partitions = Enum.map(:lists.seq(0, n_partitions - 1), fn n -> {name, n} end)

    {:ok, stream_storage_state} = init_stream(name, n_partitions, state)

    options = Keyword.put(options, :stream_storage_state, stream_storage_state)

    Enum.reduce(partitions, state, fn id, state ->
      ensure_stream(id, message, state, options)
    end)
  end

  @spec ensure_stream(stream_id(), map(), state(), Keyword.t()) :: state()
  def ensure_stream(id, message, state, options) do
    case find_stream(id, state) do
      {:ok, stream} ->
        notify_create(stream, message, state, options)

      :error ->
        create_stream(id, message, state, options)
    end
  end

  @spec find_stream(stream_id(), state()) :: {:ok, pid()} | :error
  def find_stream(id, state) do
    streams = Map.get(state, :streams, %{})
    Map.fetch(streams, id)
  end

  @spec register_stream(stream_id(), String.t(), state()) :: state()
  def register_stream(id, address, state) do
    ## TODO: maybe use address in the registry?
    case Ockam.Node.whereis(address) do
      nil ->
        raise("Stream not found on address #{address}")

      pid when is_pid(pid) ->
        streams = Map.get(state, :streams, %{})
        Map.put(state, :streams, Map.put(streams, id, pid))
    end
  end

  @spec notify_create(pid(), map(), state(), Keyword.t()) :: state()
  def notify_create(pid, message, state, options) do
    return_route = Message.return_route(message)
    StreamWorker.notify(pid, return_route, options)
    state
  end

  @spec create_stream(stream_id(), map(), state(), Keyword.t()) :: state()
  def create_stream({name, partition} = id, message, state, options) do
    return_route = Message.return_route(message)
    stream_options = Map.get(state, :stream_options, []) ++ options

    {:ok, address} =
      StreamWorker.create(
        stream_options ++
          [
            reply_route: return_route,
            stream_name: name,
            partition: partition
          ]
      )

    register_stream(id, address, state)
  end

  def create_stream_name(state) do
    random_string = "generated_" <> Base.encode16(:crypto.strong_rand_bytes(4), case: :lower)

    case find_stream({random_string, 0}, state) do
      {:ok, _} -> create_stream_name(state)
      :error -> random_string
    end
  end

  def stream_options(options) do
    stream_options = Keyword.get(options, :stream_options, [])
    storage_mod = Keyword.get(stream_options, :storage_mod, @default_storage)
    storage_options = Keyword.get(stream_options, :storage_options, [])
    Keyword.merge(stream_options, storage_mod: storage_mod, storage_options: storage_options)
  end

  def init_stream(name, n_partitions, state) do
    options = Map.fetch!(state, :stream_options)
    storage_mod = Keyword.fetch!(options, :storage_mod)
    storage_options = Keyword.fetch!(options, :storage_options)

    storage_mod.init_stream(name, n_partitions, storage_options)
  end
end
