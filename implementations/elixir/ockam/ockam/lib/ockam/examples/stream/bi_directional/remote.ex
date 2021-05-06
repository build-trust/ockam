defmodule Ockam.Example.Stream.BiDirectional.Remote do
  @moduledoc """

  Ping-pong example for bi-directional stream communication using remote subsctiption

  Use-case: integrate ockam nodes which don't implement stream protocol yet

  Pre-requisites:

  Ockam hub running with stream service and TCP listener

  Two ockam nodes "ping" and "pong":

  Expected behaviour:

  Two nodes "ping" and "pong" send messages to each other using two streams:
  "pong_topic" to send messages to "pong" node
  "ping_topic" to send messages to "ping" node

  Implementation:

  Stream service is running on the hub node

  Stream subscription service can create consumers and publishers on the hub node

  Ping and pong nodes call subscription service to get addresses to send messages to

  PROTOCOLS:

  This example using protocols to create remote consumer and publisher,
  defined in ../../../stream/client/bi_directional/subscribe.ex

  Ockam.Protocol.Stream.BiDirectional:

  name: stream_bidirectional
  request: {
    stream_name: string
    subscription_id: string
  }

  Ockam.Protocol.Stream.BiDirectional.EnsurePublisher
  name: stream_bidirectional_publisher
  request: {
    publisher_stream: :string,
    consumer_stream: :string,
    subscription_id: :string
  }

  """

  alias Ockam.Example.Stream.Ping
  alias Ockam.Example.Stream.Pong

  alias Ockam.Message

  alias Ockam.Workers.Call

  @hub_tcp %Ockam.Transport.TCPAddress{ip: {127, 0, 0, 1}, port: 4000}

  ## This should be run on PONG node.
  ## It returns a forwarding alias to use to route messages to PONG
  def init_pong() do
    ensure_tcp(5000)

    ## We run a call named "pong" to create an alias route to "pong"
    ## This should happen before creating the proper "pong"
    alias_address = register_alias("pong")
    :timer.sleep(1000)

    ## Create the actual pong worker
    {:ok, "pong"} = Pong.create(address: "pong")

    ## Call subsctiption service to create a remote consumer, which will forward
    ## messages through TCP to the pong NODE
    subscribe("pong_topic")

    alias_address
  end

  ## This should be run on PING node after PONG initialized
  ## Accepting the PONG alias address
  def run(pong_address) do
    ensure_tcp(3000)
    Ping.create(address: "ping")

    subscribe("ping_topic")

    # Call subscribe service to get remote publisher
    reply =
      Call.call(%{
        onward_route: [@hub_tcp, "stream_subscribe"],
        payload:
          Ockam.Protocol.encode_payload(
            Ockam.Protocol.Stream.BiDirectional.EnsurePublisher,
            :request,
            %{
              publisher_stream: "pong_topic",
              consumer_stream: "ping_topic",
              subscription_id: "foo"
            }
          )
      })

    ## Get the route to remote publisher
    publisher_route = Message.return_route(reply)

    ## Send message THROUGH publisher to the destination address
    send_message(publisher_route ++ [pong_address])
  end

  def send_message(route) do
    msg = %{
      onward_route: route,
      return_route: ["ping"],
      payload: "0"
    }

    Ockam.Router.route(msg)
  end

  ## Calling subscribe service to create a remote consumer
  def subscribe(stream) do
    ## Remote subscribe

    subscribe_msg = %{
      onward_route: [@hub_tcp, "stream_subscribe"],
      return_route: [],
      payload:
        Ockam.Protocol.encode_payload(Ockam.Protocol.Stream.BiDirectional, :request, %{
          stream_name: stream,
          subscription_id: "foo"
        })
    }

    Ockam.Router.route(subscribe_msg)
    ## No return yet, so just wait
    :timer.sleep(2000)
  end

  def ensure_tcp(port) do
    Ockam.Transport.TCP.create_listener(port: port, route_outgoing: true)
  end

  def register_alias(address) do
    reply =
      Call.call(
        %{
          onward_route: [@hub_tcp, "forwarding_service"],
          payload: "register"
        },
        address: address
      )

    alias_route = Message.return_route(reply)
    List.last(alias_route)
  end
end
