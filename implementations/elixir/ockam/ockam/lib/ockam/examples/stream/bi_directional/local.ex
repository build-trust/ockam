defmodule Ockam.Examples.Stream.BiDirectional.Local do
  @moduledoc """

  Ping-pong example for bi-directional stream communication using local subsctiption

  Use-case: integrate ockam nodes which implement stream protocol consumer and publisher

  Pre-requisites:

  Ockam cloud node running with stream service and TCP listener

  Two ockam nodes "ping" and "pong"

  Expected behaviour:

  Two nodes "ping" and "pong" send messages to each other using two streams:
  "pong_topic" to send messages to "pong" node
  "ping_topic" to send messages to "ping" node

  Implementation:

  Stream service is running on the cloud node

  Ping and pong nodes create local consumers and publishers to exchange messages
  """
  alias Ockam.Examples.Ping
  alias Ockam.Examples.Pong

  alias Ockam.Stream.Client.BiDirectional
  alias Ockam.Stream.Client.BiDirectional.PublisherRegistry

  alias Ockam.Transport.TCP

  def config() do
    %{
      cloud_ip: "127.0.0.1",
      cloud_port: 4000,
      service_address: "stream",
      index_address: "stream_index"
    }
  end

  def stream_options() do
    config = config()

    {:ok, cloud_ip_n} = :inet.parse_address(to_charlist(config.cloud_ip))
    tcp_address = Ockam.Transport.TCPAddress.new(cloud_ip_n, config.cloud_port)

    [
      service_route: [tcp_address, config.service_address],
      index_route: [tcp_address, config.index_address],
      partitions: 1
    ]
  end

  ## This should be run on the PONG node
  def init_pong() do
    TCP.start()
    ## PONG worker
    {:ok, "pong"} = Pong.create(address: "pong")

    ## Create a local subscription to forward pong_topic messages to local node
    subscribe("pong_topic", "pong")
  end

  def run() do
    TCP.start()

    ## PING worker
    Ping.create(address: "ping")

    ## Subscribe to response topic
    subscribe("ping_topic", "ping")

    ## Create local publisher worker to forward to pong_topic and add metadata to
    ## messages to send responses to ping_topic
    {:ok, address} = init_publisher("pong_topic", "ping_topic", "ping")

    ## Send a message THROUGH the local publisher to the remote worker
    send_message([address, "pong"])
  end

  def init_publisher(publisher_stream, consumer_stream, subscription_id) do
    BiDirectional.ensure_publisher(
      consumer_stream,
      publisher_stream,
      subscription_id,
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

  def subscribe(stream, subscription_id) do
    ## Local subscribe
    ## Create bidirectional subscription on local node
    ## using stream service configuration from stream_options
    BiDirectional.subscribe(stream, subscription_id, stream_options())

    ## This is necessary to make sure we don't spawn publisher for each message
    PublisherRegistry.start_link([])
  end
end
