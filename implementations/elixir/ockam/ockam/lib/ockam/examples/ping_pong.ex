defmodule Ockam.Examples.Ping do
  @moduledoc false
  use Ockam.Worker
  alias Ockam.Message
  alias Ockam.Worker

  require Logger

  @impl true
  def setup(options, state) do
    delay = Keyword.get(options, :delay, 50)
    {:ok, Map.put(state, :delay, delay)}
  end

  @impl true
  def handle_message(message, state) do
    # Logger.info("\nReceived message: #{inspect(message)}")

    {previous, ""} = Integer.parse(Message.payload(message))

    Logger.info("\nReceived pong for #{inspect(previous)}")

    state =
      case Map.get(state, :last, 0) do
        high when high > previous ->
          Logger.info("Duplicate pong for: #{inspect(previous)}, current: #{inspect(high)}")
          state

        _low ->
          next = previous + 1

          :timer.sleep(Map.get(state, :delay))

          reply = Message.reply(message, state.address, "#{next}")

          Logger.info("\nSend ping #{inspect(next)}")
          Worker.route(reply, state)
          Map.put(state, :last, next)
      end

    {:ok, state}
  end
end

defmodule Ockam.Examples.Pong do
  @moduledoc false

  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Worker

  require Logger

  @impl true
  def setup(options, state) do
    delay = Keyword.get(options, :delay, 50)
    {:ok, Map.put(state, :delay, delay)}
  end

  @impl true
  def handle_message(message, state) do
    reply = Message.reply(message, state.address, Message.payload(message))

    :timer.sleep(Map.get(state, :delay))

    Logger.info("\nPong\nMESSAGE: #{inspect(message)}\nREPLY: #{inspect(reply)}")
    Worker.route(reply, state)

    {:ok, state}
  end
end
