defmodule Ockam.Messaging.PipeChannel do
  @moduledoc """
  Ockam channel using pipes to deliver messages

  Can be used with different pipe implementations to get different delivery properties

  See `Ockam.Messaging.PipeChannel.Initiator` and `Ockam.Messaging.PipeChannel.Responder` for usage

  Session setup:

  Initiator is started with a route to spawner

  Initiator starts a local receiver
  Initiator sends handshake to spawner route
  handshake message return route contains receiver address

  Spawner starts a Responder with:
  return route from the handshake message
  Initiator address from the handshake message metadata

  Responder starts a local receiver
  Responder starts a sender using the return route of the handshake
  Responder sends handshake response to Initiator through local sender
  Using route: [responder_sender, initiator]

  Responder Sender forwards handshake response to Initiator Receiver

  Initiator Receiver forwards handshake response to Initiator

  Initiator takes receiver address and responder address from the handshake response metadata
  Initiator creates a route to Responder Receiver using receiver address and spawner route
  Initiator creates a local sender using this route

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

  alias Ockam.Message
  alias Ockam.Router

  @doc false
  ## Inner message is forwarded with outer address in return route
  def forward_inner(message, state) do
    [_me | onward_route] = Message.onward_route(message)
    return_route = Message.return_route(message)
    payload = Message.payload(message)

    Router.route(%{
      onward_route: onward_route,
      return_route: [state.address | return_route],
      payload: payload
    })
  end

  @doc false
  ## Outer message is forwarded through sender
  ## to other channel endpoints inner address
  def forward_outer(message, state) do
    channel_route = Map.get(state, :channel_route)

    [_me | onward_route] = Message.onward_route(message)
    return_route = Message.return_route(message)
    payload = Message.payload(message)

    sender = Map.fetch!(state, :sender)

    Router.route(%{
      onward_route: [sender | channel_route ++ onward_route],
      return_route: return_route,
      payload: payload
    })
  end

  @doc false
  def register_inner_address(state) do
    {:ok, inner_address} = Ockam.Node.register_random_address()
    Map.put(state, :inner_address, inner_address)
  end

  @doc false
  def pipe_mods(options) do
    case Keyword.fetch(options, :pipe_mods) do
      {:ok, {sender_mod, receiver_mod}} ->
        {:ok, {sender_mod, receiver_mod}}

      {:ok, pipe_mod} when is_atom(pipe_mod) ->
        {:ok, {pipe_mod.sender(), pipe_mod.receiver()}}
    end
  end
end

defmodule Ockam.Messaging.PipeChannel.Metadata do
  @moduledoc """
  Encodable data structure for pipechannel handshake metadata
  """

  defstruct [:receiver_route, :channel_route]

  @type t() :: %__MODULE__{}

  ## TODO: use proper address encoding
  @schema {:struct, [receiver_route: {:array, :data}, channel_route: {:array, :data}]}

  @spec encode(t()) :: binary()
  def encode(meta) do
    :bare.encode(meta, @schema)
  end

  @spec decode(binary()) :: t()
  def decode(data) do
    case :bare.decode(data, @schema) do
      {:ok, meta, ""} ->
        struct(__MODULE__, meta)

      other ->
        exit({:meta_decode_error, data, other})
    end
  end
end

defmodule Ockam.Messaging.PipeChannel.Initiator do
  @moduledoc """
  Pipe channel initiator.

  Using two addresses for inner and outer communication.

  Starts a local receiver and sends a handshake message to the remote spawner.
  The handshake message is sent wiht a RECEIVER address in the retourn route.

  In handshake stage:
  buffers all messages received on outer address.
  On handshake response creates a local sender using handshake metadata (receiver route) and spawner route.

  In ready stage:
  forwards messages from outer address to the sender and remote responder
  forwards messages from inner address to the onward route and traces own outer address in the return route

  Options:

  `pipe_mods` - pipe modules to use, either {sender, receiver} or a module implementing `Ockam.Messaging.Pipe`
  `spawner_route` - a route to responder spawner

  """

  use Ockam.AsymmetricWorker

  alias Ockam.Messaging.PipeChannel
  alias Ockam.Messaging.PipeChannel.Metadata

  alias Ockam.Message
  alias Ockam.Router

  @impl true
  def inner_setup(options, state) do
    spawner_route = Keyword.fetch!(options, :spawner_route)

    {:ok, {sender_mod, receiver_mod}} = PipeChannel.pipe_mods(options)

    {:ok, receiver} = receiver_mod.create([])

    send_handshake(spawner_route, receiver, state)

    {:ok,
     Map.merge(state, %{
       receiver: receiver,
       spawner_route: spawner_route,
       state: :handshake,
       sender_mod: sender_mod,
       receiver_mod: receiver_mod
     })}
  end

  @impl true
  def handle_inner_message(message, %{state: :handshake} = state) do
    payload = Message.payload(message)

    %Metadata{
      channel_route: channel_route,
      receiver_route: remote_receiver_route
    } = Metadata.decode(payload)

    spawner_route = Map.fetch!(state, :spawner_route)

    receiver_route = make_receiver_route(spawner_route, remote_receiver_route)

    sender_mod = Map.get(state, :sender_mod)
    {:ok, sender} = sender_mod.create(receiver_route: receiver_route)

    process_buffer(
      Map.merge(state, %{
        sender: sender,
        channel_route: channel_route,
        state: :ready
      })
    )
  end

  def handle_inner_message(message, %{state: :ready} = state) do
    PipeChannel.forward_inner(message, state)
    {:ok, state}
  end

  @impl true
  def handle_outer_message(message, %{state: :handshake} = state) do
    ## TODO: find a better solution than buffering
    state = buffer_message(message, state)
    {:ok, state}
  end

  def handle_outer_message(message, %{state: :ready} = state) do
    PipeChannel.forward_outer(message, state)
    {:ok, state}
  end

  defp process_buffer(state) do
    buffer = Map.get(state, :buffer, [])

    Enum.reduce(buffer, {:ok, state}, fn message, {:ok, state} ->
      handle_outer_message(message, state)
    end)
  end

  defp buffer_message(message, state) do
    buffer = Map.get(state, :buffer, [])
    Map.put(state, :buffer, buffer ++ [message])
  end

  defp make_receiver_route(spawner_route, remote_receiver_route) do
    Enum.take(spawner_route, Enum.count(spawner_route) - 1) ++ remote_receiver_route
  end

  defp send_handshake(spawner_route, receiver, state) do
    msg = %{
      onward_route: spawner_route,
      return_route: [receiver],
      payload:
        Metadata.encode(%Metadata{
          channel_route: [state.inner_address],
          receiver_route: [receiver]
        })
    }

    Router.route(msg)
  end
end

defmodule Ockam.Messaging.PipeChannel.Responder do
  @moduledoc """
  Pipe channel responder

  Using two addresses for inner and outer communication.

  Created with remote receiver route and channel route

  On start:
  creates a local receiver
  creates a sender for a remote receiver route
  sends a channel handshake confirmation through the sender
  confirmation contains local receiver address and responder inner address

  forwards messages from outer address through the sender and remote initiator
  forwards messages from inner address and traces own outer address in the return route

  Options:

  `pipe_mods` - pipe modules to use, either {sender, receiver} or an atom namespace, which has .Sender and .Receiver (e.g. `Ockam.Messaging.Ordering.Monotonic.IndexPipe`)
  `receiver_route` - route to the receiver on the initiator side, used to create a sender
  `channel_route` - route from initiator receiver to initiator, used in forwarding
  """

  use Ockam.AsymmetricWorker

  alias Ockam.Messaging.PipeChannel
  alias Ockam.Messaging.PipeChannel.Metadata

  alias Ockam.Router

  require Logger

  @impl true
  def inner_setup(options, state) do
    receiver_route = Keyword.fetch!(options, :receiver_route)
    channel_route = Keyword.fetch!(options, :channel_route)

    {:ok, {sender_mod, receiver_mod}} = PipeChannel.pipe_mods(options)

    {:ok, receiver} = receiver_mod.create([])
    {:ok, sender} = sender_mod.create(receiver_route: receiver_route)

    send_handshake_response(receiver, sender, channel_route, state)

    {:ok,
     Map.merge(state, %{
       receiver: receiver,
       sender: sender,
       channel_route: channel_route,
       sender_mod: sender_mod,
       receiver_mod: receiver_mod
     })}
  end

  @impl true
  def handle_inner_message(message, state) do
    PipeChannel.forward_inner(message, state)
  end

  @impl true
  def handle_outer_message(message, state) do
    PipeChannel.forward_outer(message, state)
  end

  defp send_handshake_response(receiver, sender, channel_route, state) do
    msg = %{
      onward_route: [sender | channel_route],
      return_route: [state.inner_address],
      payload:
        Metadata.encode(%Metadata{
          channel_route: [state.inner_address],
          receiver_route: [receiver]
        })
    }

    # Logger.info("Handshake response #{inspect(msg)}")

    Router.route(msg)
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
  use Ockam.Worker

  alias Ockam.Messaging.PipeChannel.Metadata
  alias Ockam.Messaging.PipeChannel.Responder

  alias Ockam.Message

  require Logger

  @impl true
  def setup(options, state) do
    responder_options = Keyword.fetch!(options, :responder_options)
    {:ok, Map.put(state, :responder_options, responder_options)}
  end

  @impl true
  def handle_message(message, state) do
    return_route = Message.return_route(message)
    payload = Message.payload(message)

    ## We ignore receiver route here and rely on return route tracing
    %Metadata{channel_route: channel_route} = Metadata.decode(payload)

    responder_options = Map.get(state, :responder_options)

    Responder.create(
      Keyword.merge(responder_options, receiver_route: return_route, channel_route: channel_route)
    )

    {:ok, state}
  end
end
