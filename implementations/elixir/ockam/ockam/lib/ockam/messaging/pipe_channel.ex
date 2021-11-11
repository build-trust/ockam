defmodule Ockam.Messaging.PipeChannel do
  @moduledoc """
  Ockam channel using pipes to deliver messages

  Can be used with different pipe implementations to get different delivery properties

  See `Ockam.Messaging.PipeChannel.Initiator` and `Ockam.Messaging.PipeChannel.Responder` for usage

  Session setup:

  See `Ockam.Messaging.PipeChannel.Handshake`

  Message forwarding:

  Each channel endpoint is using two addresses: INNER and OUTER.
  INNER address us used to communicate with the pipes
  OUTER address is used to communicate to other workers

  On receiving a message from OUTER address with:
  OR: [outer] ++ onward_route
  RR: return_route

  Channel endpoint sends a message with:
  OR: [local_sender, remote_endpoint] ++ onward_route
  RR: return_route

  On receiving a message from INNER address with:
  OR: [inner] ++ onward_route
  RR: return_route

  It forwards a message with:
  OR: onward_route
  RR: [outer] ++ return_route
  """

  def initiator() do
    Ockam.Messaging.PipeChannel.Initiator
  end

  def responder() do
    Ockam.Messaging.PipeChannel.Responder
  end
end
