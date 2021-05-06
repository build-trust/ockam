defmodule Ockam.Stream.Client.Consumer do
  @moduledoc false
  use Ockam.Worker
  use Ockam.Protocol.Mapping

  alias Ockam.Message
  alias Ockam.Protocol.Stream, as: StreamProtocol

  require Logger

  defstruct address: nil,
            stream_name: nil,
            stream_route: nil,
            index_route: nil,
            index: 0,
            message_handler: nil

  @type state() :: %__MODULE__{}

  @consume_limit 10
  @idle_timeout 5000

  def start(service_route, index_route, stream_name, message_handler) do
    __MODULE__.create(
      service_route: service_route,
      index_route: index_route,
      stream_name: stream_name,
      message_handler: message_handler
    )
  end

  @protocol_mapping Ockam.Protocol.Mapping.mapping([
                      {:client, StreamProtocol.Create},
                      {:client, StreamProtocol.Partitioned.Create},
                      {:client, StreamProtocol.Pull},
                      {:client, StreamProtocol.Index},
                      {:client, Ockam.Protocol.Error},
                      {:client, Ockam.Protocol.Binary}
                    ])

  @impl true
  def protocol_mapping() do
    @protocol_mapping
  end

  @impl true
  def setup(options, state) do
    service_route = Keyword.fetch!(options, :service_route)
    index_route = Keyword.fetch!(options, :index_route)
    stream_name = Keyword.fetch!(options, :stream_name)
    partitions = Keyword.fetch!(options, :partitions)

    message_handler = Keyword.fetch!(options, :message_handler)

    create_stream(service_route, stream_name, partitions, state)

    state =
      struct(__MODULE__, state)
      |> Map.put(:stream_name, stream_name)
      |> Map.put(:index_route, index_route)
      |> Map.put(:message_handler, message_handler)

    {:ok, state}
  end

  @impl true
  def handle_message(%{payload: payload} = message, state) do
    case decode_payload(payload) do
      {:ok, StreamProtocol.Create, %{stream_name: stream_name}} ->
        state = add_stream(state, stream_name, Message.return_route(message))

        request_index(state)
        {:ok, state}

      {:ok, StreamProtocol.Partitioned.Create, %{stream_name: stream_name, partition: 0}} ->
        state = add_stream(state, stream_name, Message.return_route(message))

        request_index(state)
        {:ok, state}

      {:ok, StreamProtocol.Pull, %{request_id: request_id, messages: messages}} ->
        state = messages_received(request_id, messages, state)
        {:ok, state}

      {:ok, StreamProtocol.Index, %{client_id: client_id, stream_name: stream_name, index: index}} ->
        validate_index(client_id, stream_name, state)

        start_with =
          case index do
            :undefined -> 0
            num when is_integer(num) -> num
          end

        Logger.info("Initial index #{index}: start with :#{inspect(start_with)}")
        state = consume(start_with, state)
        {:ok, state}

      {:ok, Ockam.Protocol.Error, %{reason: reason}} ->
        Logger.error("Stream error: #{inspect(reason)}")
        {:ok, state}

      other ->
        Logger.error("Unexpected message #{inspect(other)}")
        {:ok, state}
    end
  end

  ## TODO: rework the worker to do handle_info
  def handle_message(:consume, state) do
    request_messages(state)
    {:ok, state}
  end

  def request_index(state) do
    index_id = index_id(state)
    index_request = encode_payload(StreamProtocol.Index, :get, index_id)
    index_route = Map.get(state, :index_route)
    route(index_request, index_route, state)
  end

  def index_id(state) do
    %{
      client_id: Map.get(state, :address),
      stream_name: Map.get(state, :stream_name)
    }
  end

  def validate_index(client_id, stream_name, state) do
    received_id = %{client_id: client_id, stream_name: stream_name}

    case index_id(state) do
      ^received_id ->
        :ok

      other ->
        raise("Index ID #{inspect(received_id)} does not match #{inspect(other)}")
    end
  end

  def save_index(index, state) do
    index_id = index_id(state)
    index_request = encode_payload(StreamProtocol.Index, :save, Map.put(index_id, :index, index))
    index_route = Map.get(state, :index_route)
    route(index_request, index_route, state)
  end

  def consume(index, state) do
    state = Map.put(state, :index, index)
    request_messages(state)
    state
  end

  def request_messages(state) do
    next_request_id = :rand.uniform(1000)

    start_index = Map.get(state, :index, 0)

    send_pull_request(start_index, @consume_limit, next_request_id, state)
  end

  def send_pull_request(index, limit, request_id, state) do
    encoded =
      encode_payload(StreamProtocol.Pull, %{index: index, limit: limit, request_id: request_id})

    stream_route = Map.get(state, :stream_route)
    route(encoded, stream_route, state)
  end

  def messages_received(_request_id, messages, state) do
    Logger.debug(
      "consumer for #{inspect(state.stream_name)} received messages #{inspect(messages)}"
    )

    ## TODO: request id can be used with a timeout to retry if no response is received
    case messages do
      [] ->
        consume_after(@idle_timeout, state)

      _msgs ->
        Logger.info("Messages: #{inspect(messages)}")
        max_index = messages |> Enum.max_by(fn %{index: index} -> index end) |> Map.get(:index)
        commit_index = max_index + 1
        Logger.info("max index is #{max_index}: save #{commit_index}")
        save_index(commit_index, state)
        process_messages(messages, state)
        consume(commit_index, state)
    end
  end

  def consume_after(timeout, state) do
    Process.send_after(self(), :consume, timeout)
    state
  end

  def process_messages(messages, state) do
    Enum.each(messages, fn %{data: data} ->
      process_message(data, state)
    end)
  end

  def process_message(data, state) do
    message_handler = Map.get(state, :message_handler)

    try do
      message_handler.(data)
    catch
      type, reason ->
        Logger.error("Message handling error: #{inspect({type, reason})}")
    end
  end

  def add_stream(state, stream_name, stream_route) do
    Map.merge(state, %{stream_name: stream_name, stream_route: stream_route})
  end

  def create_stream(service_route, stream_name, partitions, state) do
    encoded =
      encode_payload(StreamProtocol.Partitioned.Create, %{
        stream_name: stream_name,
        partitions: partitions
      })

    route(encoded, service_route, state)
  end

  def route(payload, route, state) do
    Ockam.Router.route(%{
      onward_route: route,
      return_route: [Map.get(state, :address)],
      payload: payload
    })
  end
end
