defmodule Ockam.Kafka.Transport.Publisher do
  use Ockam.Worker

  require Logger

  alias Ockam.Stream.Storage.Kafka, as: KafkaStorage
  alias Ockam.Kafka.Transport.Address

  @default_worker :kafka_transport
  @wire_encoder_decoder Ockam.Wire.Binary.V2

  @impl true
  def setup(options, state) do
    Ockam.Kafka.ensure_kafka_worker(options, @default_worker)
    worker_name = Ockam.Kafka.worker_name(options, @default_worker)
    options = Keyword.put(options, :worker_name, worker_name)

    topic = Keyword.fetch!(options, :topic)

    state =
      Map.merge(state, %{
        stream_options: options,
        topic: topic
      })

    {:ok, state}
  end

  @impl true

  def handle_message(%{payload: _} = message, state) do
    [%Address{topic: onward_topic} | onward_route] = Ockam.Message.onward_route(message)

    return_route = Ockam.Message.return_route(message)
    payload = Ockam.Message.payload(message)

    return_topic = Map.fetch!(state, :topic)

    transport_message = %{
      payload: payload,
      onward_route: onward_route,
      return_route: [%Address{topic: return_topic} | return_route]
    }

    case Ockam.Wire.encode(@wire_encoder_decoder, transport_message) do
      {:ok, data} ->
        push_message(onward_topic, data, state)

      {:error, reason} ->
        Logger.error("Encode error: #{inspect(reason)}")
    end

    {:ok, state}
  end

  def push_message(topic, data, state) do
    stream_options = Map.fetch!(state, :stream_options)

    ## TODO: partitioning strategies
    push_to = 0

    KafkaStorage.save(topic, push_to, data, stream_options)
  end
end
