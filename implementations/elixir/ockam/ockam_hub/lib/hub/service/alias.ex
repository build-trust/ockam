defmodule Ockam.Hub.Service.Alias do
  @moduledoc """
  Alias service to register remote workers under local names.

  On message:
  creates new Ockam.Hub.Service.Alias.Forwarder workers under a random address
  passing return route of the message.

  The Forwarder worker will forward messages to the recorded route
  """

  use Ockam.Worker

  alias Ockam.Routable
  alias Ockam.Router

  require Logger

  @impl true
  def handle_message(message, state) do
    Logger.info("ALIAS service\nMESSAGE: #{inspect(message)}")
    forward_route = Routable.return_route(message)
    payload = Routable.payload(message)

    {:ok, _alias_address} =
      __MODULE__.Forwarder.create(
        forward_route: forward_route,
        registration_payload: payload
      )

    {:ok, state}
  end
end

defmodule Ockam.Hub.Service.Alias.Forwarder do
  @moduledoc """
  Forwarder worker.

  Takes two options:
  :forward_route - a route to send received messages to
  :registration_payload - a payload to send after registration to the :forward_route

  On start will send :registration_payload to the :forward_route to inform
  the registered process of its new alias

  On message will forward it to the :forward_route NOT adding the alias
  to the return route trace
  """
  use Ockam.Worker

  alias Ockam.Routable
  alias Ockam.Router

  require Logger

  @impl true
  def setup(options, state) do
    Logger.info("Created new alias for #{inspect(options)}")
    forward_route = Keyword.fetch!(options, :forward_route)
    registration_payload = Keyword.fetch!(options, :registration_payload)

    :ok = send_registration_ok(forward_route, registration_payload, state)

    {:ok, Map.put(state, :forward_route, Keyword.fetch!(options, :forward_route))}
  end

  @impl true
  def handle_message(message, %{forward_route: route} = state) do
    send_forward(route, message)

    {:ok, state}
  end

  def send_forward(route, message) do
    Logger.info("Alias forward #{inspect(message)} to #{inspect(route)}")

    Router.route(%{
      onward_route: route,
      return_route: Routable.return_route(message),
      payload: Routable.payload(message)
    })
  end

  def send_registration_ok(forward_route, registration_payload, state) do
    reply = %{
      onward_route: forward_route,
      return_route: [state.address],
      payload: registration_payload
    }

    Logger.info("REGISTER OK: #{inspect(reply)}")
    Router.route(reply)
  end
end
