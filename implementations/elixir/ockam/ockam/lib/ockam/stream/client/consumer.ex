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
            message_handler: nil,
            ready: false,
            request_timeout: nil

  @type state() :: %__MODULE__{}

  @consume_limit 10
  @idle_timeout 2_000

  @request_timeout 10_000

  def start(service_route, index_route, stream_name, message_handler) do
    __MODULE__.create(
      service_route: service_route,
      index_route: index_route,
      stream_name: stream_name,
      message_handler: message_handler
    )
  end

  def ready?(address) when is_binary(address) do
    GenServer.call(Ockam.Node.whereis(address), :check_ready)
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

    client_id = Keyword.get(options, :client_id, Map.get(state, :address))

    message_handler = Keyword.fetch!(options, :message_handler)

    state = create_stream(service_route, stream_name, partitions, state)

    state =
      struct(__MODULE__, state)
      |> Map.put(:stream_name, stream_name)
      |> Map.put(:index_route, index_route)
      |> Map.put(:message_handler, message_handler)
      |> Map.put(:client_id, client_id)

    {:ok, state}
  end

  @impl true
  def handle_message(%Ockam.Message{payload: payload} = message, state) do
    case decode_payload(payload) do
      {:ok, StreamProtocol.Create, %{stream_name: stream_name}} ->
        Logger.debug("Received create")

        state =
          state
          |> clear_request_timeout()
          |> add_stream(stream_name, Message.return_route(message))

        state = request_index(state)

        {:ok, state}

      {:ok, StreamProtocol.Partitioned.Create, %{stream_name: stream_name, partition: 0}} ->
        Logger.debug("Received create")

        state =
          state
          |> clear_request_timeout()
          |> add_stream(stream_name, Message.return_route(message))

        state = request_index(state)

        {:ok, state}

      {:ok, StreamProtocol.Pull, %{request_id: request_id, messages: messages}} ->
        Logger.debug("Pull response")
        state = clear_request_timeout(state)
        state = messages_received(request_id, messages, state)
        {:ok, state}

      {:ok, StreamProtocol.Index, %{client_id: client_id, stream_name: stream_name, index: index}} ->
        Logger.debug("Received index")
        validate_index(client_id, stream_name, state)

        ## Update index route to use tcp session
        new_index_route = Message.return_route(message)

        state =
          state
          |> clear_request_timeout()
          |> Map.put(:index_route, new_index_route)

        start_with =
          case index do
            :undefined -> 0
            num when is_integer(num) -> num
          end

        Logger.info("Initial index #{index}: start with :#{inspect(start_with)}")
        state = consume(start_with, state)
        {:ok, Map.put(state, :ready, true)}

      {:ok, Ockam.Protocol.Error, %{reason: reason}} ->
        Logger.error("Stream error: #{inspect(reason)}")
        {:ok, state}

      other ->
        Logger.error("Unexpected message #{inspect(other)}")
        {:ok, state}
    end
  end

  @impl true
  ## TODO: rework the worker to do handle_info
  def handle_info(:consume, state) do
    state = request_messages(state)
    {:noreply, state}
  end

  def handle_info(:request_timeout, state) do
    {:stop, :request_timeout, state}
  end

  @impl true
  def handle_call(:check_ready, _from, state) do
    {:reply, Map.get(state, :ready, false), state}
  end

  def request_index(state) do
    index_id = index_id(state)
    index_request = encode_payload(StreamProtocol.Index, :get, index_id)
    index_route = Map.get(state, :index_route)
    route(index_request, index_route, state)
  end

  def index_id(state) do
    %{
      client_id: Map.get(state, :client_id),
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
    route(index_request, index_route, state, :infinity)
  end

  def consume(index, state) do
    state = Map.put(state, :index, index)
    request_messages(state)
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
    Logger.debug("Send pull")
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
        max_index = messages |> Enum.max_by(fn %{index: index} -> index end) |> Map.get(:index)
        commit_index = max_index + 1

        current_index = Map.get(state, :index)

        state = process_messages(messages, state)

        case commit_index > current_index do
          true ->
            save_index(commit_index, state)
            consume(commit_index, state)

          false ->
            consume_after(@idle_timeout, state)
        end
    end
  end

  def consume_after(timeout, state) do
    Process.send_after(self(), :consume, timeout)
    state
  end

  def process_messages(messages, state) do
    Enum.reduce(messages, state, fn %{data: data}, state ->
      process_message(data, state)
    end)
  end

  def process_message(data, state) do
    message_handler = Map.get(state, :message_handler)

    try do
      case message_handler.(data, state) do
        {:ok, new_state} ->
          new_state

        :ok ->
          state

        {:error, error} ->
          ## TODO: this is a place to dead-letter?
          Logger.error("Message handling error: #{inspect(error)}")
      end
    catch
      type, reason ->
        ## TODO: this is a place to dead-letter?
        Logger.error("Message handling exception: #{inspect({type, reason})}")
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

  def route(payload, route, state, timeout \\ @request_timeout) do
    Ockam.Worker.route(
      %{
        onward_route: route,
        return_route: [Map.get(state, :address)],
        payload: payload
      },
      state
    )

    set_request_timeout(state, timeout)
  end

  def set_request_timeout(state, :infinity) do
    state
  end

  def set_request_timeout(state, timeout) do
    state = clear_request_timeout(state)
    mon_ref = Process.send_after(self(), :request_timeout, timeout)
    Map.put(state, :request_timeout, mon_ref)
  end

  def clear_request_timeout(state) do
    case Map.get(state, :request_timeout) do
      nil ->
        state

      ref ->
        Process.cancel_timer(ref)
        ## Flush the timeout message if it's already received
        receive do
          :request_timeout -> :ok
        after
          0 -> :ok
        end

        Map.put(state, :request_timeout, nil)
    end
  end
end
