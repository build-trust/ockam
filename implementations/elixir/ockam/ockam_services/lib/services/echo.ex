defmodule Ockam.Services.Echo do
  @moduledoc false

  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Worker

  require Logger

  @impl true
  def setup(options, state) do
    log_level = Keyword.get(options, :log_level, :info)
    {:ok, Map.put(state, :log_level, log_level)}
  end

  @impl true
  def handle_message(message, state) do
    reply = Message.reply(message, state.address, Message.payload(message))

    log_level = Map.get(state, :log_level, :info)
    Logger.log(log_level, "\nECHO\nMESSAGE: #{inspect(message)}\nREPLY: #{inspect(reply)}")
    Worker.route(reply, state)

    {:ok, state}
  end
end
