defmodule Ockam.Stream.Client.Publisher do
  @moduledoc false
  use Ockam.Worker
  use Ockam.Protocol.Mapping

  alias Ockam.Message
  alias Ockam.Protocol.Stream, as: StreamProtocol

  require Logger

  defstruct address: nil,
            stream_name: nil,
            stream_route: nil,
            last_message: 0,
            unconfirmed: %{},
            unsent: []

  @type state() :: %__MODULE__{}

  @protocol_mapping Ockam.Protocol.Mapping.mapping([
                      {:client, StreamProtocol.Create},
                      {:client, StreamProtocol.Partitioned.Create},
                      {:client, StreamProtocol.Push},
                      {:client, Ockam.Protocol.Error},
                      {:server, Ockam.Protocol.Binary}
                    ])

  @impl true
  def protocol_mapping() do
    @protocol_mapping
  end

  @impl true
  def setup(options, state) do
    service_route = Keyword.fetch!(options, :service_route)
    stream_name = Keyword.fetch!(options, :stream_name)
    partitions = Keyword.fetch!(options, :partitions)

    create_stream(service_route, stream_name, partitions, state)

    {:ok, struct(__MODULE__, Map.put(state, :stream_name, stream_name))}
  end

  @impl true
  def handle_message(message, state) do
    payload = Message.payload(message)

    case decode_payload(payload) do
      {:ok, StreamProtocol.Create, %{stream_name: stream_name}} ->
        state =
          state
          |> add_stream(stream_name, Message.return_route(message))
          |> send_unsent()

        {:ok, state}

      ## TODO: support multiple partitions
      {:ok, StreamProtocol.Partitioned.Create, %{stream_name: stream_name, partition: 0}} ->
        state =
          state
          |> add_stream(stream_name, Message.return_route(message))
          |> send_unsent()

        {:ok, state}

      {:ok, StreamProtocol.Push, %{status: :ok, request_id: request_id, index: index}} ->
        state = message_confirmed(request_id, index, state)
        {:ok, state}

      {:ok, StreamProtocol.Push, %{status: :error, request_id: request_id}} ->
        ## Resend doesn't change the state currently
        resend_message(request_id, state)
        {:ok, state}

      {:ok, Ockam.Protocol.Error, %{reason: reason}} ->
        Logger.error("Stream error: #{inspect(reason)}")
        {:ok, state}

      {:ok, Ockam.Protocol.Binary, data} ->
        state = send_message(data, state)
        {:ok, state}

      other ->
        Logger.error("Unexpected message #{inspect(other)}")
        {:ok, state}
    end
  end

  @spec send_message(binary(), state()) :: state()
  def send_message(data, state) do
    case initialized?(state) do
      true ->
        next = state.last_message + 1
        message = %{request_id: next, data: data}
        route_push(message, state)

        add_unconfirmed(next, message, state)

      false ->
        add_unsent(data, state)
    end
  end

  def initialized?(state) do
    case Map.get(state, :stream_route) do
      nil -> false
      _address -> true
    end
  end

  def add_stream(state, stream_name, stream_route) do
    Map.merge(state, %{stream_name: stream_name, stream_route: stream_route})
  end

  def send_unsent(state) do
    unsent = Map.get(state, :unsent, [])
    without_unsent = Map.put(state, :unsent, [])

    Enum.reduce(unsent, without_unsent, fn data, send_state ->
      send_message(data, send_state)
    end)
  end

  def add_unsent(data, state) do
    Map.update(state, :unsent, [data], fn unsent -> [data | unsent] end)
  end

  def add_unconfirmed(request_id, message, state) do
    Map.update(state, :unconfirmed, %{request_id => message}, fn unconfirmed ->
      Map.put(unconfirmed, request_id, message)
    end)
  end

  def remove_unconfirmed(request_id, state) do
    Map.update(state, :unconfirmed, %{}, fn unconfirmed -> Map.delete(unconfirmed, request_id) end)
  end

  def get_unconfirmed(request_id, state) do
    state |> Map.get(:unconfirmed, %{}) |> Map.fetch(request_id)
  end

  def resend_message(request_id, state) do
    case get_unconfirmed(request_id, state) do
      {:ok, message} ->
        route_push(message, state)

      :error ->
        :ok
    end
  end

  def message_confirmed(request_id, index, state) do
    Logger.info("Message confirmed with index #{inspect(index)}")
    remove_unconfirmed(request_id, state)
  end

  def route_push(message, state) do
    encoded = encode_payload(StreamProtocol.Push, message)
    route(encoded, Map.get(state, :stream_route), state)
  end

  def create_stream(service_route, stream_name, partitions, state) do
    Logger.info("create stream #{inspect({service_route, stream_name})}")

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
