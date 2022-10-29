defmodule Ockam.Examples.Session.Routing do
  @moduledoc """
  Simple routing session example

  Creates a spawner for sessions and establishes a routing session
  with a simple forwarding data worker.

  Usage:
  ```
  {:ok, spawner} = Ockam.Examples.Session.Routing.create_spawner() # create a responder spawner

  {:ok, initiator} = Ockam.Examples.Session.Routing.create_initiator([spawner]) # creates an initiator using a route to spawner

  Ockam.Examples.Session.Routing.run_local() # creates local spawner and initiator and sends a message through
  ```
  """

  alias Ockam.Session.Pluggable, as: Session
  alias Ockam.Session.Spawner

  alias Ockam.Examples.Session.Routing.DataWorker, as: DataWorker

  def create_spawner() do
    responder_options = [
      data_worker_mod: DataWorker,
      data_worker_options: [messages: [:message_from_options]]
    ]

    spawner_options = [
      worker_mod: Session.Responder,
      worker_options: responder_options
    ]

    Spawner.create(spawner_options)
  end

  def create_responder() do
    responder_options = [
      data_worker_mod: DataWorker,
      data_worker_options: [messages: [:message_from_options]]
    ]

    Session.Responder.create(responder_options)
  end

  def create_initiator(init_route) do
    Session.Initiator.create(
      data_worker_mod: DataWorker,
      data_worker_options: [messages: [:message_from_options_initiator]],
      init_route: init_route
    )
  end

  def run_without_spawner() do
    {:ok, responder} = create_responder()

    {:ok, responder_inner} = Ockam.AsymmetricWorker.get_inner_address(responder)

    {:ok, initiator} = create_initiator([responder_inner])

    Session.Initiator.wait_for_session(initiator)

    Ockam.Node.register_address("me")

    Ockam.Router.route(%{
      onward_route: [initiator, "me"],
      return_route: ["me"],
      payload: "Hi me!"
    })

    :sys.get_state(Ockam.Node.whereis(initiator))
  end

  def run() do
    {:ok, spawner} = create_spawner()
    {:ok, initiator} = create_initiator([spawner])

    Session.Initiator.wait_for_session(initiator)

    Ockam.Node.register_address("me")

    Ockam.Router.route(%{
      onward_route: [initiator, "me"],
      return_route: ["me"],
      payload: "Hi me!"
    })

    :sys.get_state(Ockam.Node.whereis(initiator))
  end
end
