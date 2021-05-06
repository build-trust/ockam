defmodule Ockam.Kafka.Transport.Consumer do
  use GenServer

  require Logger

  alias Ockam.Stream.Storage.Kafka, as: KafkaStorage

  def start_link(options) do
    GenServer.start_link(__MODULE__, options)
  end

  @default_worker :kafka_transport
  @default_delay 10000

  @wire_encoder_decoder Ockam.Wire.Binary.V2

  def init(options) do
    Ockam.Kafka.ensure_kafka_worker(options, @default_worker)
    worker_name = Ockam.Kafka.worker_name(options, @default_worker)
    options = Keyword.put(options, :worker_name, worker_name)

    topic = Keyword.fetch!(options, :topic)
    partitions = Keyword.get(options, :partitions, 1)

    ## TODO: move that to Ockam.Kafka
    case KafkaStorage.init_stream(topic, partitions, options) do
      {:ok, stream_options} ->
        offsets = init_offsets(topic, partitions, options)

        state = %{
          stream_options: stream_options,
          topic: topic,
          partitions: partitions,
          current_offsets: offsets
        }

        {:ok, consumer_loop(state)}

      {:error, reason} ->
        {:error, reason}
    end
  end

  def handle_info(:fetch_loop, state) do
    new_state =
      state
      |> do_fetch_loop()
      |> consumer_loop()

    {:noreply, new_state}
  end

  def do_fetch_loop(state) do
    offsets = current_offsets(state)

    messages = fetch_messages(offsets, state)
    process_messages(messages)

    last_offsets = last_message_offsets(messages, offsets)
    update_offsets(last_offsets, state)
  end

  def init_offsets(_topic, partitions, _options) do
    ## TODO: load from offset storage
    Enum.map(:lists.seq(0, partitions - 1), fn partition -> {partition, nil} end)
  end

  def current_offsets(state) do
    Map.get(state, :current_offsets)
  end

  def update_offsets(offsets, state) do
    Map.put(state, :current_offsets, offsets)
  end

  def fetch_messages(offsets, %{stream_options: options} = state) do
    topic = Map.fetch!(state, :topic)

    offsets
    |> Enum.map(fn {partition, offset} ->
      fetch_offset =
        case offset do
          nil -> 0
          val when is_integer(val) -> val + 1
        end

      case KafkaStorage.fetch(topic, partition, fetch_offset, 100, options) do
        {{:ok, messages}, _} ->
          {partition, messages}

        {{:error, reason}, _} ->
          Logger.error("Fetch error: #{inspect(reason)}")
          {partition, []}
      end
    end)
  end

  def last_message_offsets(messages, offsets) do
    messages
    |> Enum.map(fn {partition, partition_messages} ->
      old_offset =
        case List.keyfind(offsets, partition, 0) do
          nil -> nil
          {_, val} -> val
        end

      new_offset =
        partition_messages
        |> Enum.map(fn %{index: index} -> index end)
        |> Enum.max(&>=/2, fn -> old_offset end)

      {partition, new_offset}
    end)
  end

  defp fetch_delay(%{stream_options: options}) do
    Keyword.get(options, :fetch_delay, @default_delay)
  end

  def process_messages(messages) do
    messages
    |> Enum.each(fn {_partition, p_messages} ->
      p_messages
      |> Enum.each(fn %{data: message} ->
        message |> decode() |> route()
      end)
    end)
  end

  def decode(data) do
    {:ok, %{payload: _} = message} = Ockam.Wire.decode(@wire_encoder_decoder, data)
    message
  end

  def route(message) do
    # TODO: update return route to route back
    Ockam.Router.route(message)
  end

  def consumer_loop(state) do
    fetch_delay = fetch_delay(state)
    Process.send_after(self(), :fetch_loop, fetch_delay)
    state
  end
end
