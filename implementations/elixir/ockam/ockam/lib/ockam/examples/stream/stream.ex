defmodule Ockam.Examples.Stream do
  @moduledoc false

  alias Ockam.Stream.Client.Consumer
  alias Ockam.Stream.Client.Publisher

  require Logger

  def outline() do
    Ockam.Examples.Stream.run()

    Ockam.Examples.Stream.route_message("I am another message")

    Ockam.Examples.Stream.route_multiple_messages("messageNo", 100, 10)
  end

  def run() do
    init_res = init()

    route_message("Im a message")

    init_res
  end

  def stream_options() do
    config = %{
      cloud_ip: "127.0.0.1",
      cloud_port: 4000,
      service_address: "stream_kafka",
      index_address: "stream_kafka_index",
      stream_name: "my_client_stream"
    }

    {:ok, cloud_ip_n} = :inet.parse_address(to_charlist(config.cloud_ip))
    tcp_address = Ockam.Transport.TCPAddress.new(cloud_ip_n, config.cloud_port)

    %{
      service_route: [tcp_address, config.service_address],
      index_route: [tcp_address, config.index_address],
      stream_name: config.stream_name
    }
  end

  def init() do
    options = stream_options()
    Ockam.Transport.TCP.start()

    Map.merge(
      create_publisher(options.stream_name, options.service_route),
      create_consumer(options.stream_name, options.service_route, options.index_route)
    )
  end

  def create_consumer(stream_name, service_route, index_route) do
    {:ok, receiver_address} = Ockam.Examples.Stream.Receiver.create(address: "receiver")

    {:ok, consumer_address} =
      Consumer.create(
        address: "consumer",
        service_route: service_route,
        index_route: index_route,
        stream_name: stream_name,
        message_handler: fn data, _state ->
          Ockam.Router.route(%{
            onward_route: [receiver_address],
            return_route: [],
            payload: Ockam.Protocol.encode_payload(Ockam.Protocol.Binary, :request, data)
          })

          :ok
        end,
        partitions: 1
      )

    %{receiver: receiver_address, consumer: consumer_address}
  end

  def create_publisher(stream_name, service_route) do
    {:ok, publisher_address} =
      Publisher.create(
        address: "publisher",
        stream_name: stream_name,
        service_route: service_route,
        partitions: 1
      )

    %{publisher: publisher_address}
  end

  def route_message(message) do
    payload = Ockam.Protocol.encode_payload(Ockam.Protocol.Binary, :request, message)

    Ockam.Router.route(%{
      onward_route: ["publisher"],
      return_route: [],
      payload: payload
    })
  end

  def route_multiple_messages(prefix, num, delay \\ 0) do
    Enum.each(
      :lists.seq(1, num),
      fn n ->
        :timer.sleep(delay)
        route_message("#{prefix}_#{n}")
      end
    )
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
