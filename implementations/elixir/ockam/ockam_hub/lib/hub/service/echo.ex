defmodule Ockam.Hub.Service.Echo do
  @moduledoc false

  use Ockam.Worker

  alias Ockam.Routable
  alias Ockam.Router

  require Logger

  @impl true
  def handle_message(message, state) do
    reply = %{
      onward_route: Routable.return_route(message),
      return_route: [state.address],
      payload: Routable.payload(message)
    }

    Logger.info("\nECHO\nMESSAGE: #{inspect(message)}\nREPLY: #{inspect(reply)}")
    Router.route(reply)

    {:ok, state}
  end
end
