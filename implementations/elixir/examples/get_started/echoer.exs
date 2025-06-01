defmodule Echoer do
  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Worker

  @impl true
  def handle_message(message, %{address: address} = state) do
    IO.puts("Address: #{address}\t Received: #{inspect(message)}")
    Worker.route(Message.reply(message, address, Message.payload(message)), state)

    {:ok, state}
  end
end
