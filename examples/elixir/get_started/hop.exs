defmodule Hop do
  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Router

  @impl true
  def handle_message(message, state) do
    IO.puts("Address: #{state.address}\t Received: #{inspect(message)}")

    # Remove my address from beginning of onward_route
    [_ | onward_route] = Message.onward_route(message)

    # Add my address to beginning of return_route
    return_route = [state.address | Message.return_route(message)]

    :ok = Router.route(%{onward_route: onward_route, return_route: return_route, payload: Message.payload(message)})

    {:ok, state}
  end
end
