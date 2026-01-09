defmodule RendezvousWorker do
  @moduledoc """
  An Ockam Worker that acts as a rendezvous server for UDP hole punching.
  """

  use Ockam.Worker

  require Logger

  alias Ockam.{Message, Router}

  @rendezvous_node "rendezvous"

  def setup do
    __MODULE__.create(
      address: @rendezvous_node,
      attributes: %{addresses: %{}, pending_connections: []}
    )
  end

  @impl true
  def handle_message(%{payload: "my address"} = message, state) do
    [external_address, source] = message.return_route

    Logger.debug("Replying with address: #{external_address.value}")

    Router.route(Message.reply(message, state.address, "address: #{external_address.value}"))

    state = put_in(state, [:attributes, :addresses, source], external_address)
    {:ok, state}
  end

  def handle_message(%{payload: "connect"} = message, state) do
    source = message.return_route |> Enum.reverse() |> hd()
    target = message.onward_route |> Enum.reverse() |> hd()

    Logger.debug("Received connect message from #{inspect(source)} to #{inspect(target)}")

    state =
      state.attributes.addresses
      |> Map.get(target)
      |> case do
        nil ->
          Logger.debug("Target #{target} not found")
          pending = [{source, target} | state.attributes.pending_connections]
          put_in(state, [:attributes, :pending_connections], pending)

        target_address ->
          Logger.debug("Target #{target} address found: #{inspect(target_address)}")

          pending =
            if {target, source} in state.attributes.pending_connections do
              Logger.debug("Pending connection found")

              Router.route(%{
                payload: "connected",
                onward_route: [target_address, target],
                return_route: message.return_route
              })

              Router.route(%{
                payload: "connected",
                onward_route: message.return_route,
                return_route: [target_address, target]
              })

              state.attributes.pending_connections
              |> Enum.reject(&(&1 == {target, source}))
            else
              Logger.debug("Pending connection not found")

              [{source, target} | state.attributes.pending_connections]
            end

          put_in(state, [:attributes, :pending_connections], pending)
      end

    {:ok, state}
  end

  def handle_message(message, state) do
    Logger.warning("Unknown message #{inspect(message)}")

    {:ok, state}
  end
end
