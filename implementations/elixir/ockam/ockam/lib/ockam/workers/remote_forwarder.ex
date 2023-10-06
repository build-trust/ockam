defmodule Ockam.Workers.RemoteForwarder do
  @moduledoc """
  Ockam worker to handle forwarding from the Ockam Services forwarding service `Ockam.Services.Forwarding`

  On start creates a cloud forwarder in the forwarding service.
  Forwards messages from the cloud forwarder to configured route.

  Options:

  `service_route` - a route to the forwarding service
  `forward_to` - a route to forward messages from the cloud forwarder to
  `register_payload` - (defaults to "register") payload to use when registering a forwarder

  Usage:

  Create a forwarder:
  {:ok, forwarder} = RemoteForwarder.create(
    service_route: route_to_service,
    forward_to: local_route
  )

  Get the forwarding address local to the Services node:
  forwarder_address = RemoteForwarder.forwarder_address(forwarder)

  Send messages from another node:
  Ockam.Worker.route(%{onward_route: cloud_route ++ [forwarder_address], ...})

  Messages will be delivered through the cloud forwarder
  to the remote forwarder on the first node
  to the configured `local_route`
  """

  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Router
  alias Ockam.Worker

  @doc """
  Get the remote forwarder address to send messages to this worker
  """
  def forwarder_address(server) when is_binary(server) do
    forwarder_address(Ockam.Node.whereis(server))
  end

  def forwarder_address(server) when is_pid(server) do
    GenServer.call(server, :forwarder_address)
  end

  @impl true
  def setup(options, state) do
    service_route = Keyword.fetch!(options, :service_route)
    forward_to = Keyword.fetch!(options, :forward_to)
    register_payload = Keyword.get(options, :register_payload, "register")

    case register(service_route, state.address, register_payload) do
      {:ok, forwarder_address} ->
        {:ok, Map.merge(state, %{forward_to: forward_to, forwarder_address: forwarder_address})}

      {:error, err} ->
        {:error, err}
    end
  end

  @impl true
  def handle_message(message, state) do
    [_me | onward_route] = Message.onward_route(message)

    forward_to = Map.get(state, :forward_to)

    Worker.route(Message.set_onward_route(message, forward_to ++ onward_route), state)

    {:ok, state}
  end

  @impl true
  def handle_call(:forwarder_address, _from, state) do
    {:reply, Map.get(state, :forwarder_address), state}
  end

  def register(service_route, self_address, register_payload, timeout \\ 60_000) do
    # Send 'register' message to forwarding service with the own address in the return_route
    Router.route(%{
      onward_route: service_route,
      return_route: [self_address],
      payload: register_payload
    })

    # Route to remote forwarder is the return_route of the reply
    receive do
      %{onward_route: [^self_address], return_route: forwarder_route, payload: ^register_payload} ->
        {:ok, List.last(forwarder_route)}
    after
      timeout ->
        {:error, :timeout}
    end
  end
end
