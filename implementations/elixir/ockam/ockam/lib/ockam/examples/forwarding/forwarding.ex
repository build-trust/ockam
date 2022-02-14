defmodule Ockam.Examples.Forwarding do
  @moduledoc """
  Example using forwarding service on Ockam Hub

  Uasge:

  responder() - register a forarder address and wait for a message
  initiator(forarder_address) - send a message to the forwarder address

  same_node() - run both responder and initiator
  """

  alias Ockam.Examples.Forwarding.ServiceApi

  alias Ockam.Transport.TCP

  require Logger

  @hub_address Ockam.Transport.TCPAddress.new("1.node.ockam.network", 4000)

  def same_node() do
    with {:ok, forwarder_address} <- start_responder() do
      initiator(forwarder_address)
    end
  end

  def start_responder(timeout \\ 60_000) do
    notify = self()

    spawn_link(fn ->
      responder(notify)
    end)

    receive do
      {:forwarder_address, address} ->
        {:ok, address}
    after
      timeout ->
        {:error, :failed_to_init_responder}
    end
  end

  def responder(notify \\ nil) do
    TCP.start()
    forwarding_route = [@hub_address, "forwarding_service"]

    Ockam.Node.register_address("example_responder")

    with {:ok, forwarder_address} <-
           ServiceApi.register_self(forwarding_route, "example_responder") do
      Logger.info("Forwarding address: #{inspect(forwarder_address)}")

      maybe_notify(notify, forwarder_address)

      receive do
        %{onward_route: ["example_responder"]} = message ->
          Logger.info("received message through forwarding: #{inspect(message)}")
      end
    end
  end

  def initiator(forwarder_address) do
    TCP.start()
    forwarder_route = [@hub_address, forwarder_address]

    Ockam.Node.register_address("example_initiator")

    Ockam.Router.route(%{
      onward_route: forwarder_route,
      return_route: ["example_initiator"],
      payload: "Hello forward"
    })
  end

  defp maybe_notify(pid, forwarder_address) when is_pid(pid) do
    send(pid, {:forwarder_address, forwarder_address})
  end

  defp maybe_notify(_non_pid, _forwarder_address) do
    :ok
  end
end
