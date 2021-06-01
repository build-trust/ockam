defmodule Ockam.Hub.Service.Echo do
  @moduledoc false

  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Router

  require Logger

  @impl true
  def handle_message(message, state) do
    reply = Message.reply(message, state.address, Message.payload(message))

    Logger.info("\nECHO\nMESSAGE: #{inspect(message)}\nREPLY: #{inspect(reply)}")
    Router.route(reply)

    {:ok, state}
  end
end
