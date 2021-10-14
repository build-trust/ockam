defmodule Hop do
  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Router

  @impl true
  def handle_message(message, %{address: address} = state) do
    IO.puts("Address: #{address}\t Received: #{inspect(message)}")

    Router.route(%{
      # Remove my address from beginning of onward_route.
      onward_route: message |> Message.onward_route() |> List.delete_at(0),
      # Add my address to beginning of return_route.
      return_route: message |> Message.return_route() |> List.insert_at(0, address),
      # Payload remains the same.
      payload: Message.payload(message)
    })

    {:ok, state}
  end
end
