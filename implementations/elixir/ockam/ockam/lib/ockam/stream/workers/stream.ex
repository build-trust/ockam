defmodule Ockam.Stream.Workers.Stream do
  @moduledoc false

  use Ockam.Worker
  use Ockam.Protocol.Mapping

  alias Ockam.Protocol.Stream, as: StreamProtocol

  require Logger

  @type request() :: binary()
  @type state() :: map()

  def notify(server, return_route, options) do
    GenServer.cast(server, {:notify, return_route, options})
  end

  @protocol_mapping Ockam.Protocol.Mapping.mapping([
                      {:server, StreamProtocol.Create},
                      {:server, StreamProtocol.Partitioned.Create},
                      {:server, StreamProtocol.Push},
                      {:server, StreamProtocol.Pull},
                      {:server, Ockam.Protocol.Error}
                    ])

  @impl true
  def protocol_mapping() do
    @protocol_mapping
  end

  @impl true
  def handle_cast({:notify, return_route, options}, state) do
    init_protocol = init_protocol(options)
    reply_init(state.stream_name, state.partition, return_route, init_protocol, state)
    {:noreply, state}
  end

  @impl true
  def setup(options, state) do
    reply_route = Keyword.fetch!(options, :reply_route)
    stream_name = Keyword.fetch!(options, :stream_name)
    partition = Keyword.fetch!(options, :partition)

    storage_mod = Keyword.fetch!(options, :storage_mod)

    storage_options = Keyword.fetch!(options, :storage_options)

    init_protocol = init_protocol(options)

    stream_storage_state = Keyword.get(options, :stream_storage_state, nil)

    storage =
      case storage_mod.init_partition(
             stream_name,
             partition,
             stream_storage_state,
             storage_options
           ) do
        {:ok, storage_state} ->
          {storage_mod, storage_state}

        {:error, error} ->
          raise(
            "Unable to create storage: #{
              inspect({storage_mod, stream_name, partition, storage_options})
            }. Reason: #{inspect(error)}"
          )
      end

    state =
      Map.merge(state, %{
        reply_route: reply_route,
        stream_name: stream_name,
        partition: partition,
        storage: storage
      })

    reply_init(stream_name, partition, reply_route, init_protocol, state)

    {:ok, state}
  end

  defp init_protocol(options) do
    Keyword.get(options, :protocol, StreamProtocol.Create)
  end

  @impl true
  def handle_message(%{payload: payload, return_route: return_route}, state) do
    case decode_payload(payload) do
      {:ok, type, data} ->
        handle_data(type, data, return_route, state)

      {:error, err} ->
        handle_decode_error(err, return_route, state)
    end
  end

  def handle_decode_error(err, return_route, state) do
    Logger.error("Error decoding request: #{inspect(err)}")
    error_reply = encode_error("Invalid request")
    send_reply(error_reply, return_route, state)
  end

  def handle_data(StreamProtocol.Push, push_request, return_route, state) do
    # Logger.info("Push message #{inspect(push_request)}")
    %{request_id: id, data: data} = push_request
    {result, state} = save_message(data, state)
    reply_push_confirm(result, id, return_route, state)
    {:ok, state}
  end

  def handle_data(StreamProtocol.Pull, pull_request, return_route, state) do
    # Logger.info("Pull request #{inspect(pull_request)}")
    %{request_id: request_id, index: index, limit: limit} = pull_request

    case fetch_messages(index, limit, state) do
      {{:ok, messages}, new_state} ->
        reply_pull_response(messages, request_id, return_route, state)
        {:ok, new_state}

      {{:error, err}, new_state} ->
        Logger.error("Error fetching messages #{inspect(err)}")
        ## TODO: reply error with request id
        reply_pull_response([], request_id, return_route, state)
        {:ok, new_state}
    end
  end

  ## Storage:

  @spec init_storage(state()) :: {:ok | {:error, any()}, state()}
  def init_storage(state) do
    stream_name = Map.get(state, :stream_name)
    partition = Map.get(state, :partition)

    with_storage(state, fn storage_mod, storage_state ->
      storage_mod.create_stream(stream_name, partition, storage_state)
    end)
  end

  @spec save_message(any(), state()) :: {{:ok, integer()} | {:error, any()}, state()}
  def save_message(data, state) do
    stream_name = Map.get(state, :stream_name)
    partition = Map.get(state, :partition)

    with_storage(state, fn storage_mod, storage_state ->
      storage_mod.save(stream_name, partition, data, storage_state)
    end)
  end

  @spec fetch_messages(integer(), integer(), state()) ::
          {{:ok, [%{index: integer(), data: any()}]} | {:error, any()}, state()}
  def fetch_messages(index, limit, state) do
    stream_name = Map.get(state, :stream_name)
    partition = Map.get(state, :partition)

    with_storage(state, fn storage_mod, storage_state ->
      storage_mod.fetch(stream_name, partition, index, limit, storage_state)
    end)
  end

  def with_storage(state, fun) do
    {storage_mod, storage_state} = get_storage(state)
    {result, new_storage_state} = fun.(storage_mod, storage_state)
    {result, put_storage(new_storage_state, state)}
  end

  def get_storage(state) do
    Map.fetch!(state, :storage)
  end

  def put_storage(new_storage_state, state) do
    Map.update!(state, :storage, fn {mod, _old_storage_state} ->
      {mod, new_storage_state}
    end)
  end

  ## Replies

  def reply_init(stream_name, partition, reply_route, protocol, state) do
    Logger.info("INIT stream #{inspect({stream_name, partition, reply_route})}")
    init_payload = encode_init(stream_name, partition, protocol)
    send_reply(init_payload, reply_route, state)
  end

  def reply_push_confirm(result, id, return_route, state) do
    push_confirm = encode_push_confirm(result, id)
    send_reply(push_confirm, return_route, state)
  end

  def reply_pull_response(messages, request_id, return_route, state) do
    response = encode_pull_response(messages, request_id)
    send_reply(response, return_route, state)
  end

  defp send_reply(data, reply_route, state) do
    :ok =
      Ockam.Router.route(%{
        onward_route: reply_route,
        return_route: [state.address],
        payload: data
      })
  end

  ### Encode helpers

  def encode_init(stream_name, partition, init_protocol) do
    encode_payload(init_protocol, %{stream_name: stream_name, partition: partition})
  end

  def encode_push_confirm({:ok, index}, id) do
    encode_payload(StreamProtocol.Push, %{status: :ok, request_id: id, index: index})
  end

  def encode_push_confirm({:error, error}, id) do
    Logger.error("Error saving message: #{inspect(error)}")

    encode_payload(StreamProtocol.Push, %{status: :error, request_id: id, index: 0})
  end

  def encode_pull_response(messages, request_id) do
    encode_payload(StreamProtocol.Pull, %{request_id: request_id, messages: messages})
  end

  def encode_error(reason) do
    encode_payload(Ockam.Protocol.Error, %{reason: reason})
  end
end
