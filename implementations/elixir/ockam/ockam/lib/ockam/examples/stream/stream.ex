defmodule Ockam.Examples.Stream do
  @moduledoc false

  alias Ockam.Examples.Stream.ConsumerWorker
  alias Ockam.Examples.Stream.PublisherProxy

  require Logger

  @tcp %Ockam.Transport.TCPAddress{ip: {127, 0, 0, 1}, port: 4000}
  @service_route [@tcp, "stream_service"]
  @index_route [@tcp, "stream_index_service"]

  def run() do
    ensure_tcp()
    {:ok, receiver_address} = Ockam.Examples.Stream.Receiver.create(address: "receiver")

    stream_name = "my_stream"

    {:ok, publisher_address} =
      PublisherProxy.create(
        address: "publisher",
        stream_name: stream_name,
        service_route: @service_route
      )

    {:ok, consumer_address} =
      ConsumerWorker.create(
        address: "consumer",
        service_route: @service_route,
        index_route: @index_route,
        stream_name: stream_name,
        receiver: receiver_address
      )

    route_message("HELLO!!")

    %{receiver: receiver_address, publisher: publisher_address, consumer: consumer_address}
  end

  ## Ockam.Examples.Stream.route_message("Im a message")

  def route_message(message) do
    payload = Ockam.MessageProtocol.encode_payload(Ockam.Protocol.Binary, :request, message)

    Ockam.Router.route(%{
      onward_route: ["publisher"],
      return_route: [],
      payload: payload
    })
  end

  ## Ockam.Examples.Stream.route_multiple_messages("messageNo", 10)

  def route_multiple_messages(prefix, num) do
    Enum.each(:lists.seq(1, num), fn n -> route_message("#{prefix}_#{n}") end)
  end

  def ensure_tcp() do
    Ockam.Transport.TCP.create_listener(port: 3000, route_outgoing: true)
  end
end

defmodule Ockam.Examples.Stream.Receiver do
  @moduledoc false
  use Ockam.Worker
  use Ockam.MessageProtocol

  require Logger

  @impl true
  def protocol_mapping() do
    Ockam.Protocol.server(Ockam.Protocol.Binary)
  end

  @impl true
  def handle_message(%{payload: payload}, state) do
    case decode_payload(payload) do
      {:ok, "binary", data} ->
        Logger.info("Received a message: #{inspect(data)}")

      other ->
        Logger.info("Unexpected message #{inspect(other)}")
    end

    {:ok, state}
  end
end
