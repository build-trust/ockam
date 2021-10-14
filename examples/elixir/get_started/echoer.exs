defmodule Echoer do
  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Router

  @impl true
  def handle_message(message, %{address: address} = state) do
    IO.puts("Address: #{address}\t Received: #{inspect(message)}")

    Router.route(%{
      # Make return_route of incoming message, onward_route of outgoing message.
      onward_route: Message.return_route(message),
      # Make my address the the return_route of the new message.
      return_route: [address],
      # Echo back the same payload.
      payload: Message.payload(message)
    })

    {:ok, state}
  end
end
