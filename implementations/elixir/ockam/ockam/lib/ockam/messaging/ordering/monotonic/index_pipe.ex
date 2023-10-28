defmodule Ockam.Messaging.Ordering.Monotonic.IndexPipe do
  @moduledoc """
  Monotonicly ordered pipe using indexing to enforce ordering

  See `Ockam.Messaging.Ordering.IndexPipe.Sender` and
  `Ockam.Messaging.Ordering.Monotonic.IndexPipe.Receiver`
  """
  @behaviour Ockam.Messaging.Pipe

  @doc "Get sender module"
  def sender() do
    Ockam.Messaging.IndexPipe.Sender
  end

  @doc "Get receiver module"
  def receiver() do
    Ockam.Messaging.Ordering.Monotonic.IndexPipe.Receiver
  end
end

defmodule Ockam.Messaging.Ordering.Monotonic.IndexPipe.Receiver do
  @moduledoc """
  Receiver side of monotonic ordered pipe using indexing to enforce ordering
  Maintains a monotonic sent message index

  Receives wrapped messages from the sender, unwraps them and only forwards
  if the message index is higher then the monotonic index.
  After sending a message updates the index to the message index

  """
  use Ockam.Worker

  alias Ockam.Messaging.IndexPipe.Wrapper

  require Logger

  @impl true
  def handle_message(indexed_message, state) do
    case Wrapper.unwrap_message(Ockam.Message.payload(indexed_message)) do
      {:ok, index, message} ->
        case index_valid?(index, state) do
          true ->
            Ockam.Worker.route(message, state)
            {:ok, Map.put(state, :current_index, index)}

          false ->
            Logger.warning(
              "Cannot send message #{inspect(message)} with index #{inspect(index)}. Current index: #{inspect(current_index(state))}"
            )

            {:ok, state}
        end

      other ->
        Logger.error(
          "Unable to decode indexed message: #{inspect(indexed_message)}, reason: #{inspect(other)}"
        )

        {:error, :unable_to_decode_message}
    end
  end

  defp index_valid?(index, state) do
    index > current_index(state)
  end

  defp current_index(state) do
    Map.get(state, :current_index, 0)
  end
end
