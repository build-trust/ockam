defmodule Ockam.Example.Stream.Ping do
  @moduledoc false
  use Ockam.Worker
  alias Ockam.Message
  alias Ockam.Router

  require Logger

  @impl true
  def handle_message(message, state) do
    # Logger.info("\nReceived message: #{inspect(message)}")

    {previous, ""} = Integer.parse(Message.payload(message))

    Logger.info("\nReceived pong fo #{inspect(previous)}")

    state =
      case Map.get(state, :last, 0) do
        high when high > previous ->
          Logger.info("Duplicate pong for: #{inspect(previous)}, current: #{inspect(high)}")
          state

        _low ->
          next = previous + 1

          :timer.sleep(50)

          reply = %{
            onward_route: Message.return_route(message),
            return_route: [state.address],
            payload: "#{next}"
          }

          Logger.info("\nSend ping #{inspect(next)}")
          Router.route(reply)
          Map.put(state, :last, next)
      end

    {:ok, state}
  end
end

defmodule Ockam.Example.Stream.Pong do
  @moduledoc false

  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Router

  require Logger

  @impl true
  def handle_message(message, state) do
    reply = %{
      onward_route: Message.return_route(message),
      return_route: [state.address],
      payload: Message.payload(message)
    }

    Logger.info("\nPong\nMESSAGE: #{inspect(message)}\nREPLY: #{inspect(reply)}")
    Router.route(reply)

    {:ok, state}
  end
end
