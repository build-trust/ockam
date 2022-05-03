defmodule Ockam.Services.Forwarding do
  @moduledoc """
  Alias service to register remote workers under local names.

  On message:
  creates new Ockam.Services.Alias.Forwarder workers under a random address
  passing return route of the message.

  The Forwarder worker will forward messages to the recorded route
  """

  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Router

  require Logger

  @impl true
  def handle_message(message, state) do
    Logger.info("ALIAS service\nMESSAGE: #{inspect(message)}")
    forward_route = Message.return_route(message)
    payload = Message.payload(message)

    {:ok, _alias_address} =
      __MODULE__.Forwarder.create(
        forward_route: forward_route,
        registration_payload: payload
      )

    {:ok, state}
  end
end

defmodule Ockam.Services.Forwarding.Forwarder do
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

  alias Ockam.Message
  alias Ockam.Router

  require Logger

  @impl true
  def address_prefix(_options), do: "FWD_"

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
    [_me | onward_route] = Message.onward_route(message)

    route_to_forward = route ++ onward_route
    Logger.info("Alias forward #{inspect(message)} to #{inspect(route_to_forward)}")

    Router.route(Message.forward(message, route_to_forward))

    {:ok, state}
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
