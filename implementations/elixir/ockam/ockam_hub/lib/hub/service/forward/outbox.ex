defmodule Ockam.Hub.Service.Forward.Outbox do
  @moduledoc false

  use Ockam.Worker

  alias Ockam.Routable
  alias Ockam.Router

  @impl true
  def setup(options, state) do
    inbox_address = Keyword.get(options, :inbox_address)
    state = Map.put(state, :inbox_address, inbox_address)
    {:ok, state}
  end

  @impl true
  def handle_message(message, state) do
    onward = Message.onward_route(message)
    onward = Enum.drop_while(onward, fn a -> a === state.address end)

    return = [state.inbox_address | Message.return_route(message)]
    payload = Message.payload(message)

    forward = %{onward_route: onward, return_route: return, payload: payload}

    with :ok <- Router.route(forward) do
      {:ok, state}
    end
  end
end
