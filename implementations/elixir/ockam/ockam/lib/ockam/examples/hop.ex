defmodule Ockam.Examples.Hop do
  @moduledoc false

  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Router

  @impl true
  def handle_message(message, state) do
    Router.route(Message.forward(message) |> Message.trace(state.address))

    {:ok, state}
  end
end
