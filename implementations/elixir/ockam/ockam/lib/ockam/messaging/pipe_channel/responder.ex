defmodule Ockam.Messaging.PipeChannel.Responder do
  @moduledoc """
  Pipe channel responder

  A session responder using `Ockam.Messaging.PipeChannel.Handshake` for handshake
  and `Ockam.Messaging.PipeChannel.Simple` for data exchange

  Options:

  `pipe_mod` - pipe module
  `sender_options` - options for sender
  `receiver_options` - options for receiver
  """

  alias Ockam.Messaging.PipeChannel

  alias Ockam.Session.Pluggable, as: Session

  def create(options) do
    init_message = Keyword.get(options, :init_message)

    pipe_mod = Keyword.fetch!(options, :pipe_mod)
    sender_options = Keyword.get(options, :sender_options, [])
    receiver_options = Keyword.get(options, :receiver_options, [])

    address_options = Keyword.take(options, [:address, :inner_address])

    Session.Responder.create(
      address_options ++
        [
          init_message: init_message,
          worker_mod: PipeChannel.Simple,
          handshake: PipeChannel.Handshake,
          handshake_options: [
            pipe_mod: pipe_mod,
            sender_options: sender_options,
            receiver_options: receiver_options
          ]
        ]
    )
  end
end

defmodule Ockam.Messaging.PipeChannel.Spawner do
  @moduledoc """
  Pipe channel receiver spawner

  On message spawns a channel receiver
  with remote route as a remote receiver route
  and channel route taken from the message metadata

  Options:

  `responder_options` - additional options to pass to the responder
  """

  def create(options) do
    ## TODO: addresses for other workers
    address_options = Keyword.take(options, [:address, :inner_address])
    responder_options = Keyword.fetch!(options, :responder_options)

    Ockam.Session.Spawner.create(
      address_options ++
        [
          worker_mod: Ockam.Messaging.PipeChannel.Responder,
          worker_options: responder_options
        ]
    )
  end
end
