defmodule Ockam.Messaging.PipeChannel.Initiator do
  @moduledoc """
  Pipe channel initiator.

  A session initiator using `Ockam.Messaging.PipeChannel.Handshake` for handshake
  and `Ockam.Messaging.PipeChannel.Simple` for data exchange

  Options:

  `init_route` - init route for the session
  `pipe_mod` - pipe module
  `sender_options` - options for sender
  `receiver_options` - options for receiver
  """

  alias Ockam.Messaging.PipeChannel

  alias Ockam.Session.Pluggable, as: Session

  def create(options) do
    init_route = Keyword.fetch!(options, :init_route)

    pipe_mod = Keyword.fetch!(options, :pipe_mod)
    sender_options = Keyword.get(options, :sender_options, [])
    receiver_options = Keyword.get(options, :receiver_options, [])

    Session.Initiator.create(
      init_route: init_route,
      data_worker_mod: PipeChannel.Simple,
      data_worker_options: [],
      handshake_mod: PipeChannel.Handshake,
      handshake_options: [
        pipe_mod: pipe_mod,
        sender_options: sender_options,
        receiver_options: receiver_options
      ]
    )
  end

  ## TODO: solve duplication with Session.Initiator.create_and_wait
  def create_and_wait(options, interval \\ 50, timeout \\ 5000) do
    with {:ok, address} <- create(options),
         :ok <- Session.Initiator.wait_for_session(address, interval, timeout) do
      {:ok, address}
    end
  end
end
