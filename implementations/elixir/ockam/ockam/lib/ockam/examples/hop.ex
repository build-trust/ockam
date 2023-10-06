defmodule Ockam.Examples.Hop do
  @moduledoc false

  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Worker

  @impl true
  def handle_message(message, state) do
    Worker.route(Message.forward(message) |> Message.trace(state.address), state)

    {:ok, state}
  end
end
