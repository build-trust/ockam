defmodule Hop do
  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Worker

  @impl true
  def handle_message(message, %{address: address} = state) do
    IO.puts("Address: #{address}\t Received: #{inspect(message)}")

    ## Forward message to the next address and trace current address
    ## in return route.
    forwarded_message = Message.forward(message) |> Message.trace(address)

    Worker.route(forwarded_message, state)

    {:ok, state}
  end
end
