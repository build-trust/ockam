defmodule Echoer do
  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Router

  @impl true
  def handle_message(message, state) do
    IO.puts("Address: #{state.address}\t Received: #{inspect(message)}")

    r = %{onward_route: Message.return_route(message), return_route: [state.address], payload: Message.payload(message)}
    :ok = Router.route(r)

    {:ok, state}
  end
end
