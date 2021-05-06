defmodule Ockam.Examples.Stream do
  @moduledoc false

  alias Ockam.Stream.Client.Consumer
  alias Ockam.Stream.Client.Publisher

  require Logger

  @tcp %Ockam.Transport.TCPAddress{ip: {127, 0, 0, 1}, port: 4000}

  def run_internal(name_prefix \\ "") do
    workers =
      create_workers([@tcp, "stream_service"], [@tcp, "stream_index_service"], name_prefix)

    route_message("HI!", name_prefix)
    workers
  end

  def run_kafka(name_prefix \\ "") do
    workers =
      create_workers(
        [@tcp, "kafka_stream_service"],
        [@tcp, "kafka_stream_index_service"],
        name_prefix
      )

    route_message("HI!", name_prefix)
    workers
  end

  def create_workers(service_route, index_route, name_prefix \\ "") do
    ensure_tcp()

    Map.merge(
      create_publisher(service_route, name_prefix),
      create_consumer(service_route, index_route, name_prefix)
    )
  end

  def create_consumer(service_route, index_route, name_prefix \\ "") do
    {:ok, receiver_address} =
      Ockam.Examples.Stream.Receiver.create(address: name_prefix <> "receiver")

    stream_name = name_prefix <> "example_stream"

    {:ok, consumer_address} =
      Consumer.create(
        address: name_prefix <> "consumer",
        service_route: service_route,
        index_route: index_route,
        stream_name: stream_name,
        message_handler: fn data ->
          Ockam.Router.route(%{
            onward_route: [receiver_address],
            return_route: [],
            payload: Ockam.Protocol.encode_payload(Ockam.Protocol.Binary, :request, data)
          })
        end,
        partitions: 2
      )

    %{receiver: receiver_address, consumer: consumer_address}
  end

  def create_publisher(service_route, name_prefix \\ "") do
    stream_name = name_prefix <> "example_stream"

    {:ok, publisher_address} =
      Publisher.create(
        address: name_prefix <> "publisher",
        stream_name: stream_name,
        service_route: service_route,
        partitions: 2
      )

    %{publisher: publisher_address}
  end

  def create_kafka_publisher(name_prefix \\ "") do
    create_publisher([@tcp, "kafka_stream_service"], name_prefix)
  end

  def create_kafka_consumer(name_prefix \\ "") do
    create_consumer(
      [@tcp, "kafka_stream_service"],
      [@tcp, "kafka_stream_index_service"],
      name_prefix
    )
  end

  ## Ockam.Examples.Stream.create_kafka_consumer("consumer_only_")
  ## Ockam.Examples.Stream.create_kafka_publisher("publisher_only_")

  ## Ockam.Examples.Stream.route_message("Im a message")

  ## Ockam.Examples.Stream.route_multiple_messages("messageNo", 1000, 700, "publisher_only")

  ## Ockam.Examples.Stream.route_multiple_messages("messageNo", 1000, 700)

  def route_message(message, name_prefix \\ "") do
    payload = Ockam.Protocol.encode_payload(Ockam.Protocol.Binary, :request, message)

    Ockam.Router.route(%{
      onward_route: [name_prefix <> "publisher"],
      return_route: [],
      payload: payload
    })
  end

  def route_multiple_messages(prefix, num, delay \\ 0, name_prefix \\ "") do
    Enum.each(
      :lists.seq(1, num),
      fn n ->
        :timer.sleep(delay)
        route_message("#{prefix}_#{n}", name_prefix)
      end
    )
  end

  def ensure_tcp() do
    Ockam.Transport.TCP.create_listener(port: 3000, route_outgoing: true)
  end
end

defmodule Ockam.Examples.Stream.Receiver do
  @moduledoc false
  use Ockam.Worker
  use Ockam.Protocol.Mapping

  require Logger

  @protocol_mapping Ockam.Protocol.Mapping.server(Ockam.Protocol.Binary)
  @impl true
  def protocol_mapping() do
    @protocol_mapping
  end

  @impl true
  def handle_message(%{payload: payload}, state) do
    case decode_payload(payload) do
      {:ok, Ockam.Protocol.Binary, data} ->
        Logger.info("Received a message: #{inspect(data)}")

      other ->
        Logger.info("Unexpected message #{inspect(other)}")
    end

    {:ok, state}
  end
end
