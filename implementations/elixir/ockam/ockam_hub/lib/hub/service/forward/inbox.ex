defmodule Ockam.Hub.Service.Forward.Inbox do
  @moduledoc false

  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Router

  require Logger

  def outbox_address(inbox_address), do: inbox_address <> "_outbox"

  @impl true
  def setup(options, state) do
    forward_route = Keyword.get(options, :forward_route, [])
    state = Map.put(state, :forward_route, forward_route)
    state = Map.put(state, :outbox_address, outbox_address(state.address))
    {:ok, state}
  end

  @impl true
  def handle_message(message, state) do
    return_route = Message.return_route(message)

    forward = %{
      onward_route: state.forward_route,
      return_route: [state.outbox_address | return_route],
      payload: Message.payload(message)
    }

    with :ok <- Router.route(forward) do
      Logger.info("FORWADED_MESSAGE: #{inspect(forward)}")
      {:ok, state}
    end
  end
end
