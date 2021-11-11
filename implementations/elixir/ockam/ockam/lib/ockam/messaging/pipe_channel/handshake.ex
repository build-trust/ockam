defmodule Ockam.Messaging.PipeChannel.Handshake do
  @moduledoc """
  Pipe channel handshake implementation

  1.
  Initiator creates a receiver and sends own inner address and receiver address
  in the handshake message.
  Return route of the handshake message contains a route to initiator receiver

  2.
  Responder creates a sender based on return route and saves the initiator address
  Responder creates a receiver and sends a handshake with own inner address and
  receiver address

  3.
  Initiator creates a sender using init route and responder receiver address
  and saves the responder address

  Handshake message format is in `Ockam.Messaging.PipeChannel.Metadata`
  """
  @behaviour Ockam.Session.Handshake

  alias Ockam.Message
  alias Ockam.Messaging.PipeChannel.Metadata

  require Logger

  def init(handshake_options, state) do
    Logger.info("Handshake init #{inspect(handshake_options)} #{inspect(state)}")
    init_route = Map.fetch!(state, :init_route)

    pipe_mod = Keyword.fetch!(handshake_options, :pipe_mod)
    receiver_mod = pipe_mod.receiver()
    receiver_options = Keyword.get(handshake_options, :receiver_options, [])

    {:ok, receiver} = receiver_mod.create(receiver_options)

    handshake_msg = %{
      onward_route: init_route,
      return_route: [state.handshake_address],
      payload:
        Metadata.encode(%Metadata{
          channel_route: [state.worker_address],
          receiver_route: [receiver]
        })
    }

    {:next, handshake_msg, Map.put(state, :receiver, receiver)}
  end

  def handle_initiator(handshake_options, message, state) do
    payload = Message.payload(message)

    %Metadata{
      channel_route: channel_route,
      receiver_route: remote_receiver_route
    } = Metadata.decode(payload)

    init_route = Map.fetch!(state, :init_route)

    receiver_route = make_receiver_route(init_route, remote_receiver_route)

    pipe_mod = Keyword.fetch!(handshake_options, :pipe_mod)
    sender_mod = pipe_mod.sender()
    sender_options = Keyword.get(handshake_options, :sender_options, [])

    {:ok, sender} =
      sender_mod.create(Keyword.merge([receiver_route: receiver_route], sender_options))

    ## TODO: replace sender and channel_route with a single route
    {:ready, [sender: sender, channel_route: channel_route], state}
  end

  def handle_responder(handshake_options, message, state) do
    payload = Message.payload(message)

    ## We ignore receiver route here and rely on return route tracing
    %Metadata{channel_route: channel_route, receiver_route: remote_receiver_route} =
      Metadata.decode(payload)

    return_route = Message.return_route(message)

    receiver_route = make_receiver_route(return_route, remote_receiver_route)

    sender_options = Keyword.get(handshake_options, :sender_options, [])
    receiver_options = Keyword.get(handshake_options, :receiver_options, [])

    pipe_mod = Keyword.fetch!(handshake_options, :pipe_mod)
    sender_mod = pipe_mod.sender()
    receiver_mod = pipe_mod.receiver()

    {:ok, receiver} = receiver_mod.create(receiver_options)

    {:ok, sender} =
      sender_mod.create(Keyword.merge([receiver_route: receiver_route], sender_options))

    response = %{
      onward_route: return_route,
      return_route: [state.handshake_address],
      payload:
        Metadata.encode(%Metadata{
          channel_route: [state.worker_address],
          receiver_route: [receiver]
        })
    }

    {:ready, response, [sender: sender, channel_route: channel_route], state}
  end

  defp make_receiver_route(init_route, remote_receiver_route) do
    Enum.take(init_route, Enum.count(init_route) - 1) ++ remote_receiver_route
  end
end
