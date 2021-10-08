defmodule Ockam.Examples.Hop do
  @moduledoc false

  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Router

  @impl true
  def handle_message(message, state) do
    [_self | forward] = Message.onward_route(message)

    Router.route(%{
      onward_route: forward,
      return_route: [state.address | Message.return_route(message)],
      payload: Message.payload(message)
    })

    {:ok, state}
  end
end
