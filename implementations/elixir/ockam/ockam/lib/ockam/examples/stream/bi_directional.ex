defmodule Ockam.Example.Stream.BiDirectional do
  @moduledoc false

  alias Ockam.Example.Stream.Ping
  alias Ockam.Example.Stream.Pong

  alias Ockam.Stream.Client.BiDirectional

  @hub_tcp %Ockam.Transport.TCPAddress{ip: {127, 0, 0, 1}, port: 4000}

  def init_ping() do
    ensure_tcp(3000)
    Ping.create(address: "ping")
    subscribe("ping_topic")
  end

  def init_pong() do
    ensure_tcp(5000)
    Pong.create(address: "pong")
    subscribe("pong_topic")
  end

  def subscribe(stream) do
    BiDirectional.subscribe(stream_name: stream, stream_options: stream_options())
  end

  def stream_options() do
    [
      service_route: [@hub_tcp, "stream_service"],
      index_route: [@hub_tcp, "stream_index_service"],
      partitions: 1
    ]
  end

  def run() do
    init_ping()
    {:ok, address} = init_publisher("pong_topic", "ping_topic")
    send_message(address)
  end

  def init_publisher(publisher_stream, consumer_stream) do
    BiDirectional.ensure_publisher(
      consumer_stream: consumer_stream,
      publisher_stream: publisher_stream,
      stream_options: stream_options()
    )
  end

  def send_message(publisher_address) do
    msg = %{
      onward_route: [
        publisher_address,
        "pong"
      ],
      return_route: ["ping"],
      payload: "0"
    }

    Ockam.Router.route(msg)
  end

  def ensure_tcp(port) do
    Ockam.Transport.TCP.create_listener(port: port, route_outgoing: true)
  end
end
