defmodule Ockam.Services.Tracing do
  @moduledoc """
  Tracing service.

  Create watchers - workers to intercept traffic

  Watchers will forward all traffic without changes
  but send a copy of the payload to the tracing route.
  """

  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Router

  require Logger

  @impl true
  def handle_message(message, state) do
    Logger.info("TRACING service\nMESSAGE: #{inspect(message)}")
    tracing_route = Message.return_route(message)
    payload = Message.payload(message)

    {:ok, _alias_address} =
      __MODULE__.Watcher.create(
        tracing_route: tracing_route,
        reply: payload
      )

    {:ok, state}
  end
end

defmodule Ockam.Services.Tracing.Watcher do
  @moduledoc """
  Tracing watcher.

  Upon creation sends a message to the tracing_route to communicate its address

  On message will send a copy of the payload to the tracing_route
  """

  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Router

  require Logger

  @impl true
  def setup(options, state) do
    Logger.info("Created new watcher for #{inspect(options)}")
    tracing_route = Keyword.fetch!(options, :tracing_route)
    reply = Keyword.fetch!(options, :reply)

    state = Map.put(state, :tracing_route, tracing_route)

    Logger.info("REGISTER OK: #{inspect(reply)}")
    send_trace(reply, state)

    {:ok, state}
  end

  @impl true
  def handle_message(message, state) do
    route_further(message, state)

    Logger.info("Tracing #{inspect(message)} to #{inspect(state.tracing_route)}")
    send_trace(Message.payload(message), state)

    {:ok, state}
  end

  def route_further(message, %{address: address}) do
    ## TODO: use helpers for those things
    onward_route =
      case Message.onward_route(message) do
        [^address | onward_route] -> onward_route
        onward_route -> onward_route
      end

    req = Message.forward_trace(message, onward_route, address)

    Router.route(req)
  end

  def send_trace(payload, %{tracing_route: tracing_route, address: address}) do
    Router.route(%{
      onward_route: tracing_route,
      return_route: [address],
      payload: payload
    })
  end
end
