defmodule Echoer do
  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Router

  @impl true
  def handle_message(message, %{address: address} = state) do
    IO.puts("Address: #{address}\t Received: #{inspect(message)}")
    Router.route(Message.reply(message, address, Message.payload(message)))

    {:ok, state}
  end
end
