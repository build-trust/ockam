defmodule Ockam.Messaging.PipeChannel.Simple do
  @moduledoc """
  Simple implementation of pipe channel.
  Does not manage the session.
  Requires a known address to the local pipe sender and remote channel end

  Using two addresses for inner and outer communication.

  forwards messages from outer address to the sender and remote channel
  forwards messages from inner address to the onward route and traces own outer address in the return route

  Options:

  `sender` - address of the sender worker
  `channel_route` - route from remote receiver to remote channel end
  """
  use Ockam.AsymmetricWorker

  alias Ockam.Message

  alias Ockam.Worker

  @impl true
  def inner_setup(options, state) do
    sender = Keyword.fetch!(options, :sender)
    channel_route = Keyword.fetch!(options, :channel_route)

    {:ok, Map.merge(state, %{sender: sender, channel_route: channel_route})}
  end

  @impl true
  def handle_inner_message(message, state) do
    forward_inner(message, state)
    {:ok, state}
  end

  @impl true
  def handle_outer_message(message, state) do
    forward_outer(message, state)
    {:ok, state}
  end

  @doc false
  ## Inner message is forwarded with outer address in return route
  def forward_inner(message, state) do
    message = Message.forward(message) |> Message.trace(state.address)
    Worker.route(message, state)
  end

  @doc false
  ## Outer message is forwarded through sender
  ## to other channel endpoints inner address
  def forward_outer(message, state) do
    channel_route = Map.fetch!(state, :channel_route)

    [_me | onward_route] = Message.onward_route(message)

    sender = Map.fetch!(state, :sender)

    message = Message.set_onward_route(message, [sender | channel_route ++ onward_route])

    Worker.route(message, state)
  end
end
