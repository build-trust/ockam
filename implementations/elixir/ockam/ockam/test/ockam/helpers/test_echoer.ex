defmodule Ockam.Tests.Helpers.Echoer do
  @moduledoc false
  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Worker

  require Logger

  @impl true
  def handle_message(message, state) do
    reply = Message.reply(message, state.address, Message.payload(message))

    Worker.route(reply, state)
    {:ok, state}
  end
end
