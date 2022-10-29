defmodule Ockam.Examples.Session.CountTo do
  @moduledoc false

  alias Ockam.Session.Pluggable, as: Session
  alias Ockam.Session.Spawner

  alias Ockam.Examples.Session.CountTo.DataWorker, as: DataWorker

  def create_spawner() do
    responder_options = [
      data_worker_mod: DataWorker,
      handshake_mod: Ockam.Examples.Session.CountTo.Handshake,
      handshake_options: [count_to: 10]
    ]

    spawner_options = [
      worker_mod: Session.Responder,
      worker_options: responder_options
    ]

    Spawner.create(spawner_options)
  end

  def create_initiator(init_route) do
    Session.Initiator.create(
      data_worker_mod: DataWorker,
      init_route: init_route,
      handshake_mod: Ockam.Examples.Session.CountTo.Handshake,
      handshake_options: [count_to: 10]
    )
  end

  def run() do
    {:ok, spawner} = create_spawner()
    {:ok, initiator} = create_initiator([spawner])

    Session.Initiator.wait_for_session(initiator)

    Ockam.Node.register_address("me")

    Ockam.Router.route(%{
      onward_route: [initiator, "me"],
      return_route: ["me"],
      payload: ""
    })

    :sys.get_state(Ockam.Node.whereis(initiator))
  end
end
