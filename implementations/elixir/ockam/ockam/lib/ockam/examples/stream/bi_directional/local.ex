defmodule Ockam.Example.Stream.BiDirectional.Local do
  @moduledoc """

  Ping-pong example for bi-directional stream communication using local subsctiption

  Use-case: integrate ockam nodes which implement stream protocol consumer and publisher

  Pre-requisites:

  Ockam hub running with stream service and TCP listener

  Two ockam nodes "ping" and "pong"

  Expected behaviour:

  Two nodes "ping" and "pong" send messages to each other using two streams:
  "pong_topic" to send messages to "pong" node
  "ping_topic" to send messages to "ping" node

  Implementation:

  Stream service is running on the hub node

  Ping and pong nodes create local consumers and publishers to exchange messages
  """

  alias Ockam.Example.Stream.Ping
  alias Ockam.Example.Stream.Pong

  alias Ockam.Stream.Client.BiDirectional
  alias Ockam.Stream.Client.BiDirectional.PublisherRegistry

  @hub_tcp %Ockam.Transport.TCPAddress{ip: {127, 0, 0, 1}, port: 4000}

  ## This should be run on the PONG node
  def init_pong() do
    ensure_tcp(5000)
    ## PONG worker
    {:ok, "pong"} = Pong.create(address: "pong")

    ## Create a local subscription to forward pong_topic messages to local node
    subscribe("pong_topic")
  end

  def run() do
    ensure_tcp(3000)

    ## PING worker
    Ping.create(address: "ping")

    ## Subscribe to response topic
    subscribe("ping_topic")

    ## Create local publisher worker to forward to pong_topic and add metadata to
    ## messages to send responses to ping_topic
    {:ok, address} = init_publisher("pong_topic", "ping_topic")

    ## Send a message THROUGH the local publisher to the remote worker
    send_message([address, "pong"])
  end

  def init_publisher(publisher_stream, consumer_stream) do
    BiDirectional.ensure_publisher(
      consumer_stream,
      publisher_stream,
      stream_options()
    )
  end

  def send_message(onward_route) do
    msg = %{
      onward_route: onward_route,
      return_route: ["ping"],
      payload: "0"
    }

    Ockam.Router.route(msg)
  end

  def ensure_tcp(port) do
    Ockam.Transport.TCP.create_listener(port: port, route_outgoing: true)
  end

  def stream_options() do
    [
      service_route: [@hub_tcp, "stream_service"],
      index_route: [@hub_tcp, "stream_index_service"],
      partitions: 1
    ]
  end

  def subscribe(stream) do
    ## Local subscribe
    ## Create bidirectional subscription on local node
    ## using stream service configuration from stream_options
    BiDirectional.subscribe(stream, stream_options())

    ## This is necessary to make sure we don't spawn publisher for each message
    PublisherRegistry.start_link([])
  end
end
