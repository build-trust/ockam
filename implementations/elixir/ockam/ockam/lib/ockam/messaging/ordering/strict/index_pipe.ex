defmodule Ockam.Messaging.Ordering.Strict.IndexPipe do
  @moduledoc """
  Strictly ordered pipe using indexing to enforce ordering

  See `Ockam.Messaging.Ordering.IndexPipe.Sender` and
  `Ockam.Messaging.Ordering.Strict.IndexPipe.Receiver`
  """

  @behaviour Ockam.Messaging.Pipe

  @doc "Get sender module"
  def sender() do
    Ockam.Messaging.IndexPipe.Sender
  end

  @doc "Get receiver module"
  def receiver() do
    Ockam.Messaging.Ordering.Strict.IndexPipe.Receiver
  end
end

defmodule Ockam.Messaging.Ordering.Strict.IndexPipe.Receiver do
  @moduledoc """
  Receiver side of strictly ordered pipe using indexing to enforce ordering
  Maintains a sent message index and a send queue.

  Receives wrapped messages from the sender
  if the message index is current+1 - message is sent.
  if the message index is lower then the current+1 - message is ignored
  if the message index is higher then current+1 - message is put in the send queue

  When message with current+1 index is received - messages from the send queue are processed

  After sending a message updates the current index to the message index

  """
  use Ockam.Worker

  alias Ockam.Messaging.IndexPipe.Wrapper

  require Logger

  @impl true
  def handle_message(indexed_message, state) do
    case Wrapper.unwrap_message(Ockam.Message.payload(indexed_message)) do
      {:ok, index, message} ->
        case compare_index(index, state) do
          :low ->
            Logger.warning("Ignoring message #{inspect(message)} with index: #{inspect(index)}")
            {:ok, state}

          :high ->
            Logger.warning("Enqueue message #{inspect(message)} with index: #{inspect(index)}")
            {:ok, enqueue_message(index, message, state)}

          :next ->
            {:ok, send_message(index, message, state)}
        end

      other ->
        Logger.error(
          "Unable to decode indexed message: #{inspect(indexed_message)}, reason: #{inspect(other)}"
        )

        {:error, :unable_to_decode_message}
    end
  end

  defp compare_index(index, state) do
    next_index = current_index(state) + 1

    case index do
      ^next_index -> :next
      high when high > next_index -> :high
      low when low < next_index -> :low
    end
  end

  def send_message(index, message, state) do
    Ockam.Worker.route(message, state)
    state = Map.put(state, :current_index, index)
    process_queue(state)
  end

  defp process_queue(state) do
    next_index = current_index(state) + 1
    queue = queue(state)

    case Map.pop(queue, next_index) do
      {nil, _queue} ->
        state

      {message, rest} ->
        state = Map.put(state, :queue, rest)
        send_message(next_index, message, state)
    end
  end

  def enqueue_message(index, message, state) do
    queue = queue(state)

    case Map.get(queue, index) do
      nil ->
        :ok

      val ->
        Logger.debug(
          "Duplicate message: #{inspect(message)} overrides #{inspect(val)} for index #{inspect(index)}"
        )
    end

    Map.put(state, :queue, Map.put(queue, index, message))
  end

  defp queue(state) do
    Map.get(state, :queue, %{})
  end

  defp current_index(state) do
    Map.get(state, :current_index, 0)
  end
end
