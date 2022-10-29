defmodule Ockam.Examples.Session.PipeChannel do
  @moduledoc """
  Pipe channel session example
  Using the session handshake module PipeChannel.Handshake
  to create a pipe channel
  """

  alias Ockam.Session.Pluggable, as: Session
  alias Ockam.Session.Spawner

  alias Ockam.Messaging.Delivery.ResendPipe
  alias Ockam.Messaging.PipeChannel

  require Logger

  def create_spawner() do
    spawner_options = [
      worker_mod: Session.Responder,
      worker_options: [
        data_worker_mod: PipeChannel.Simple,
        handshake_mod: PipeChannel.Handshake,
        handshake_options: [
          pipe_mod: ResendPipe,
          sender_options: [confirm_timeout: 200]
        ]
      ]
    ]

    Spawner.create(spawner_options)
  end

  def create_responder() do
    responder_options = [
      data_worker_mod: PipeChannel.Simple,
      handshake_mod: PipeChannel.Handshake,
      handshake_options: [
        pipe_mod: ResendPipe,
        sender_options: [confirm_timeout: 200]
      ]
    ]

    Session.Responder.create(responder_options)
  end

  def create_initiator(init_route) do
    Session.Initiator.create_and_wait(
      init_route: init_route,
      data_worker_mod: PipeChannel.Simple,
      data_worker_options: [],
      handshake_mod: PipeChannel.Handshake,
      handshake_options: [
        pipe_mod: ResendPipe,
        sender_options: [confirm_timeout: 200]
      ]
    )
  end

  def run_without_spawner() do
    {:ok, responder} = create_responder()

    {:ok, responder_inner} = Ockam.AsymmetricWorker.get_inner_address(responder)

    {:ok, initiator} = create_initiator([responder_inner])

    Ockam.Node.register_address("me")

    Ockam.Router.route(%{
      onward_route: [initiator, "me"],
      return_route: ["me"],
      payload: "Ping me!"
    })

    receive do
      %{return_route: return_route} = message ->
        Logger.info("Received message #{inspect(message)}")

        Ockam.Router.route(%{
          onward_route: return_route,
          return_route: ["me"],
          payload: "Pong me!"
        })
    after
      2000 ->
        raise "Did not receive ping"
    end

    receive do
      %{return_route: _return_route} = message ->
        Logger.info("Received message #{inspect(message)}")
    after
      2000 ->
        raise "Did not receive pong"
    end

    :sys.get_state(Ockam.Node.whereis(initiator))
  end

  def run() do
    {:ok, spawner} = create_spawner()
    {:ok, initiator} = create_initiator([spawner])

    Ockam.Node.register_address("me")

    Ockam.Router.route(%{
      onward_route: [initiator, "me"],
      return_route: ["me"],
      payload: "Hi me!"
    })

    :sys.get_state(Ockam.Node.whereis(initiator))
  end
end
