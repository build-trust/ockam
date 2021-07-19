defmodule Ockam.Hub.Service.Alias do
  @moduledoc """
  Alias service to register remote workers under user-provided local names.

  Same as Ockam.Hub.Service.Forwarding but accepts the address to register as
  and may return errors

  Payload encoding:
  request: :string ("REG:<name>" or "DEL:<name>")
  response: :string ("OK", "ALREADY_STARTED" or "ERROR")
  """

  use Ockam.Worker

  alias Ockam.Bare.Extended, as: BareExtended

  alias Ockam.Message
  alias Ockam.Router

  require Logger

  @impl true
  def handle_message(message, state) do
    Logger.info("ALIAS service\nMESSAGE: #{inspect(message)}")
    forward_route = Message.return_route(message)
    payload = Message.payload(message)

    case BareExtended.decode(payload, :string) do
      {:ok, "REG:" <> name} ->
        case Ockam.Hub.Service.Forwarding.Forwarder.create(
               forward_route: forward_route,
               reply: "OK",
               address: make_address(name)
             ) do
          {:ok, _address} ->
            :ok

          {:error, {:already_started, _}} ->
            Router.route(%{
              onward_route: forward_route,
              return_route: [state[:address]],
              payload: "ALREADY_STARTED"
            })

          {:error, reason} ->
            Logger.error("Unable to create an alias: #{inspect(reason)}")

            Router.route(%{
              onward_route: forward_route,
              return_route: [state[:address]],
              payload: "ERROR"
            })
        end

      {:ok, "DEL:" <> name} ->
        Ockam.Node.stop(make_address(name))
        Router.route(%{
          onward_route: forward_route,
          return_route: [state[:address]],
          payload: "OK"
        })

      {:error, error} ->
        Logger.error("Unable to parse registration request: #{inspect(error)}")

        Router.route(%{
          onward_route: forward_route,
          return_route: [state[:address]],
          payload: "ERROR"
        })
    end

    {:ok, state}
  end

  def make_address(name) do
    "ALIAS_" <> name
  end
end

defmodule Ockam.Hub.Service.Forwarding do
  @moduledoc """
  Forwarding service to register remote workers under local names.

  On message:
  creates new Ockam.Hub.Service.Forwarding.Forwarder workers under a random address
  passing return route of the message.

  The Forwarder worker will forward messages to the recorded route
  """

  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Router

  require Logger

  @impl true
  def handle_message(message, state) do
    Logger.info("FORWARDING service\nMESSAGE: #{inspect(message)}")
    forward_route = Message.return_route(message)
    payload = Message.payload(message)

    {:ok, _address} = __MODULE__.Forwarder.create(forward_route: forward_route, reply: payload)

    {:ok, state}
  end
end

defmodule Ockam.Hub.Service.Forwarding.Forwarder do
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
  def setup(options, state) do
    Logger.info("Created new alias for #{inspect(options)}")
    forward_route = Keyword.fetch!(options, :forward_route)
    reply = Keyword.fetch!(options, :reply)

    :ok = send_registration_ok(forward_route, reply, state)

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
      return_route: Message.return_route(message),
      payload: Message.payload(message)
    })
  end

  def send_registration_ok(forward_route, reply, state) do
    reply = %{
      onward_route: forward_route,
      return_route: [state.address],
      payload: reply
    }

    Logger.info("REGISTER OK: #{inspect(reply)}")
    Router.route(reply)
  end
end
