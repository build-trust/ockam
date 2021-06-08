defmodule Ockam.Example.Stream.BiDirectional.SecureChannel do
  @moduledoc """

  Ping-pong example for bi-directional stream communication using local subsctiption

  Use-case: integrate ockam nodes which implement stream protocol consumer and publisher

  Pre-requisites:

  Ockam hub running with stream service and TCP listener

  Two ockam nodes "ping" and "pong"

  Expected behaviour:

  Two nodes "ping" and "pong" send messages to each other using two streams:
  "sc_listener_topic" to send messages to "pong" node
  "sc_initiator_topic" to send messages to "ping" node

  Implementation:

  Stream service is running on the hub node

  Ping and pong nodes create local consumers and publishers to exchange messages
  """
  alias Ockam.SecureChannel
  alias Ockam.Vault
  alias Ockam.Vault.Software, as: SoftwareVault

  alias Ockam.Example.Stream.Ping
  alias Ockam.Example.Stream.Pong

  alias Ockam.Stream.Client.BiDirectional
  alias Ockam.Stream.Client.BiDirectional.PublisherRegistry

  require Logger

  ## Ignore no local return for secure channel
  @dialyzer :no_return

  def config(n) do
    %{
      hub_ip: "13.64.73.230",
      # hub_ip: "127.0.0.1",
      hub_port: 4000,
      service_address: "stream_demo#{n}_service",
      index_address: "stream_demo#{n}_index"
    }
  end

  def main(args) do
    n = String.to_integer(Enum.at(args, 0))
    secure_channel_listener(n)

    secure_channel(n)

    receive do
      :stop -> :ok
    end
  end

  def secure_channel_listener(n \\ 0) do
    ensure_tcp(5000 + n)
    ## PONG worker
    {:ok, "pong"} = Pong.create(address: "pong")

    create_secure_channel_listener()

    ## Create a local subscription to forward pong_topic messages to local node
    subscribe("sc_listener_topic_k_i#{n}", "pong#{n}", n)
  end

  def secure_channel(n \\ 0) do
    ensure_tcp(3000 + n)

    ## PING worker
    Ping.create(address: "ping")

    ## Subscribe to response topic
    subscribe("sc_initiator_topic_k_i#{n}", "ping#{n}", n)

    ## Create local publisher worker to forward to pong_topic and add metadata to
    ## messages to send responses to ping_topic
    {:ok, publisher} =
      init_publisher("sc_listener_topic_k_i#{n}", "sc_initiator_topic_k_i#{n}", n)

    {:ok, channel} = create_secure_channel([publisher, "SC_listener"])

    ## Send a message THROUGH the local publisher to the remote worker
    send_message([channel, "pong"], ["ping"], "0")
  end

  defp create_secure_channel_listener() do
    {:ok, vault} = SoftwareVault.init()
    {:ok, identity} = Vault.secret_generate(vault, type: :curve25519)

    SecureChannel.create_listener(
      vault: vault,
      identity_keypair: identity,
      address: "SC_listener"
    )
  end

  defp create_secure_channel(route_to_listener) do
    {:ok, vault} = SoftwareVault.init()
    {:ok, identity} = Vault.secret_generate(vault, type: :curve25519)

    {:ok, c} =
      SecureChannel.create(route: route_to_listener, vault: vault, identity_keypair: identity)

    wait(fn -> SecureChannel.established?(c) end)
    {:ok, c}
  end

  defp wait(fun) do
    case fun.() do
      true ->
        :ok

      false ->
        :timer.sleep(100)
        wait(fun)
    end
  end

  def init_publisher(publisher_stream, consumer_stream, n) do
    BiDirectional.ensure_publisher(
      consumer_stream,
      publisher_stream,
      stream_options(n)
    )
  end

  def send_message(onward_route, return_route, payload) do
    msg = %{
      onward_route: onward_route,
      return_route: return_route,
      payload: payload
    }

    Ockam.Router.route(msg)
  end

  def ensure_tcp(port) do
    Ockam.Transport.TCP.create_listener(port: port, route_outgoing: true)
  end

  def subscribe(stream, subscription_id, n) do
    ## Local subscribe
    ## Create bidirectional subscription on local node
    ## using stream service configuration from stream_options
    {:ok, consumer} = BiDirectional.subscribe(stream, subscription_id, stream_options(n))

    wait(fn ->
      # Logger.info("Consumer: #{consumer} ready?")
      ready = Ockam.Stream.Client.Consumer.ready?(consumer)
      # Logger.info("#{ready}")
      ready
    end)

    ## This is necessary to make sure we don't spawn publisher for each message
    PublisherRegistry.start_link([])
  end

  def stream_options(n) do
    config = config(n)

    {:ok, hub_ip_n} = :inet.parse_address(to_charlist(config.hub_ip))
    tcp_address = %Ockam.Transport.TCPAddress{ip: hub_ip_n, port: config.hub_port}

    # {:ok, tcp_client} = Ockam.Transport.TCP.Client.create(destination: tcp_address)

    # Logger.info("TCP client: #{inspect(tcp_client)}")

    [
      service_route: [tcp_address, config.service_address],
      index_route: [tcp_address, config.index_address],
      partitions: 1
    ]
  end
end
