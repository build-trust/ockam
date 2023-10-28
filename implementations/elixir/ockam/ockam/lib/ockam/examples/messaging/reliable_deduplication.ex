defmodule Ockam.Examples.Messaging.ReliableDeduplication do
  @moduledoc """
  Example of combining reliable delivery with index-ordering deduplication.

  Such combination allows to get low message loss with high uniqueness of messages
  as long as pipes and channels are available and have no errors
  """

  alias Ockam.Examples.Messaging.Filter
  alias Ockam.Examples.Messaging.Shuffle

  alias Ockam.Examples.Ping
  alias Ockam.Examples.Pong

  alias Ockam.Messaging.Delivery.ResendPipe
  alias Ockam.Messaging.Ordering.Strict.IndexPipe

  alias Ockam.Messaging.PipeChannel

  alias Ockam.Workers.PubSubSubscriber

  alias Ockam.Transport.TCP.RecoverableClient

  ## Local examole
  ## Create filter and shuffle forwarders
  ## Run reliable delivery channel over filter and shuffle
  ## Wrap reliable channel into index ordering channel to deduplicate messages
  ## Send ping-pong through this combined channel
  def local() do
    ## Intermediate
    {:ok, filter} = Filter.create(address: "filter")
    {:ok, shuffle} = Shuffle.create(address: "shuffle")

    Ockam.Node.register_address("me")

    ## Pong
    {:ok, resend_spawner} =
      PipeChannel.Spawner.create(
        responder_options: [pipe_mod: ResendPipe, sender_options: [confirm_timeout: 200]]
      )

    {:ok, ord_spawner} = PipeChannel.Spawner.create(responder_options: [pipe_mod: IndexPipe])
    {:ok, "pong"} = Pong.create(address: "pong", delay: 500)

    ## Create resend channel through filter and shuffle
    {:ok, "ping"} = Ping.create(address: "ping", delay: 500)

    {:ok, resend_channel} =
      PipeChannel.Initiator.create_and_wait(
        pipe_mod: ResendPipe,
        init_route: [filter, shuffle, resend_spawner],
        sender_options: [confirm_timeout: 200]
      )

    {:ok, _ord_channel} =
      PipeChannel.Initiator.create_and_wait(
        pipe_mod: IndexPipe,
        init_route: [resend_channel, ord_spawner]
      )
  end

  def run_local() do
    {:ok, channel} = local()
    start_ping_pong(channel)
  end

  def cloud_responder() do
    Ockam.Transport.TCP.start()

    Ockam.Node.register_address("me")

    {:ok, "pong"} = Pong.create(address: "pong", delay: 500)

    {:ok, client} = RecoverableClient.create(destination: {"localhost", 4000})

    {:ok, _subscription} =
      PubSubSubscriber.create(
        pub_sub_route: [client, "pub_sub"],
        name: "responder",
        topic: "responder"
      )

    {:ok, "resend_receiver"} = ResendPipe.receiver().create(address: "resend_receiver")

    {:ok, "resend_sender"} =
      ResendPipe.sender().create(
        address: "resend_sender",
        confirm_timeout: 200,
        receiver_route: [client, "pub_sub_t_initiator", "resend_receiver"]
      )

    {:ok, "resend_channel"} =
      PipeChannel.Simple.create(
        address: "resend_channel",
        inner_address: "resend_channel_inner",
        sender: "resend_sender",
        channel_route: ["resend_channel_inner"]
      )

    {:ok, "ord_spawner"} =
      PipeChannel.Spawner.create(
        address: "ord_spawner",
        responder_options: [pipe_mod: IndexPipe]
      )
  end

  def cloud_initiator() do
    {:ok, "ping"} = Ping.create(address: "ping", delay: 500)

    {:ok, client} = RecoverableClient.create(destination: {"localhost", 4000})

    {:ok, _subscription} =
      PubSubSubscriber.create(
        pub_sub_route: [client, "pub_sub"],
        name: "initiator",
        topic: "initiator"
      )

    {:ok, "resend_receiver"} = ResendPipe.receiver().create(address: "resend_receiver")

    {:ok, "resend_sender"} =
      ResendPipe.sender().create(
        address: "resend_sender",
        confirm_timeout: 200,
        receiver_route: [client, "pub_sub_t_responder", "resend_receiver"]
      )

    {:ok, "resend_channel"} =
      PipeChannel.Simple.create(
        address: "resend_channel",
        inner_address: "resend_channel_inner",
        sender: "resend_sender",
        channel_route: ["resend_channel_inner"]
      )

    {:ok, _ord_channel} =
      PipeChannel.Initiator.create_and_wait(
        pipe_mod: IndexPipe,
        init_route: ["resend_channel", "ord_spawner"]
      )
  end

  def run_cloud_initiator() do
    {:ok, channel} = cloud_initiator()
    start_ping_pong(channel)
  end

  def send_messages(route, n_messages \\ 20) do
    Enum.each(1..n_messages, fn n ->
      Ockam.Router.route(%{
        onward_route: route ++ ["me"],
        return_route: ["me"],
        payload: "Msg #{n}"
      })
    end)
  end

  def start_ping_pong(channel) do
    ## Start ping-pong
    Ockam.Router.route(%{
      onward_route: [channel, "pong"],
      return_route: ["ping"],
      payload: "0"
    })
  end
end
