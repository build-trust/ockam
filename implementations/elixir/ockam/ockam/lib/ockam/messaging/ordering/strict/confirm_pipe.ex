defmodule Ockam.Messaging.Ordering.Strict.ConfirmPipe do
  @moduledoc """
  Ockam pipe with receive queue and confirmations.
  Next message is processed only after previous one was confirmed.

  NOTE: Confirm pipe should go over backtraceable route
  NOTE: Confirm pipe doesn't handle message loss. I will get stuck on missing confirm

  NOTE: Confirm pipe does not deduplicate messages
  """
  ## TODO: experiment with call-style waiting for confirm
  @behaviour Ockam.Messaging.Pipe

  def sender() do
    Ockam.Messaging.Ordering.Strict.ConfirmPipe.Sender
  end

  def receiver() do
    Ockam.Messaging.ConfirmPipe.Receiver
  end
end

defmodule Ockam.Messaging.Ordering.Strict.ConfirmPipe.Sender do
  @moduledoc """
  Confirm pipe sender.
  Started with receiver route

  When message is forwarded, the worker waits for a confirmation.
  Additional messages received before confirmation are put in the receive queue.
  After confirmation is received - next message from the queue is sent

  Confirmations are received on the INNER address

  Options:

  `receiver_route` - a route to receiver
  """
  use Ockam.AsymmetricWorker

  alias Ockam.Message

  alias Ockam.Messaging.ConfirmPipe.Wrapper

  @impl true
  def inner_setup(options, state) do
    receiver_route = Keyword.fetch!(options, :receiver_route)

    {:ok, Map.merge(state, %{receiver_route: receiver_route, waiting_confirm: false, queue: []})}
  end

  @impl true
  def handle_inner_message(message, state) do
    case is_valid_confirm?(message, state) do
      true ->
        confirm(state)

      false ->
        {:error, {:unknown_inner_message, message}}
    end
  end

  @impl true
  def handle_outer_message(message, state) do
    case waiting_confirm?(state) do
      true ->
        enqueue_message(message, state)

      false ->
        send_message(message, state)
    end
  end

  def is_valid_confirm?(message, state) do
    payload = Message.payload(message)
    {:ok, ref, ""} = :bare.decode(payload, :uint)

    case Map.get(state, :send_ref) do
      current_ref when current_ref <= ref ->
        true

      other_ref ->
        Logger.warning(
          "Received confirm for ref #{inspect(ref)}, current ref is #{inspect(other_ref)}"
        )

        false
    end
  end

  def waiting_confirm?(state) do
    Map.get(state, :waiting_confirm, false)
  end

  def enqueue_message(message, state) do
    queue = Map.get(state, :queue, [])
    {:ok, Map.put(state, :queue, queue ++ [message])}
  end

  def send_message(message, state) do
    receiver_route = Map.get(state, :receiver_route)
    forwarded_message = Message.forward(message)

    {ref, state} = bump_send_ref(state)
    {:ok, wrapped_message} = Wrapper.wrap_message(forwarded_message, ref)

    Ockam.Worker.route(
      %{
        onward_route: receiver_route,
        return_route: [state.inner_address],
        payload: wrapped_message
      },
      state
    )

    {:ok, Map.put(state, :waiting_confirm, true)}
  end

  def bump_send_ref(state) do
    ref = Map.get(state, :send_ref, 0) + 1
    {ref, Map.put(state, :send_ref, ref)}
  end

  def confirm(state) do
    queue = Map.get(state, :queue, [])

    case queue do
      [message | rest] ->
        send_message(message, Map.put(state, :queue, rest))

      [] ->
        {:ok, Map.put(state, :waiting_confirm, false)}
    end
  end
end
