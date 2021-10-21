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
    Ockam.Messaging.Ordering.Strict.ConfirmPipe.Receiver
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
  """
  use Ockam.AsymmetricWorker

  alias Ockam.Message

  @impl true
  def inner_setup(options, state) do
    receiver_route = Keyword.fetch!(options, :receiver_route)

    {:ok, Map.merge(state, %{receiver_route: receiver_route, waiting_confirm: false, queue: []})}
  end

  @impl true
  def handle_inner_message(message, state) do
    case is_confirm?(message) do
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
        queue_message(message, state)

      false ->
        send_message(message, state)
    end
  end

  def is_confirm?(message) do
    ## TODO: do we need some payload here?
    ## Revisit when we have message types
    Message.payload(message) == ""
  end

  def waiting_confirm?(state) do
    Map.get(state, :waiting_confirm, false)
  end

  def queue_message(message, state) do
    queue = Map.get(state, :queue, [])
    {:ok, Map.put(state, :queue, queue ++ [message])}
  end

  def send_message(message, state) do
    ## TODO: do we need to wrap the message?
    receiver_route = Map.get(state, :receiver_route)
    [_me | onward_route] = Message.onward_route(message)

    forwarded_message = %{
      onward_route: onward_route,
      return_route: Message.return_route(message),
      payload: Message.payload(message)
    }

    {:ok, wrapped_message} = Ockam.Wire.encode(forwarded_message)

    Ockam.Router.route(%{
      onward_route: receiver_route,
      return_route: [state.inner_address],
      payload: wrapped_message
    })

    {:ok, Map.put(state, :waiting_confirm, true)}
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

defmodule Ockam.Messaging.Ordering.Strict.ConfirmPipe.Receiver do
  @moduledoc """
  Confirm receiver sends a confirm message for every message received
  """
  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Router

  require Logger

  @impl true
  def handle_message(message, state) do
    return_route = Message.return_route(message)
    wrapped_message = Message.payload(message)

    case Ockam.Wire.decode(wrapped_message) do
      {:ok, message} ->
        Router.route(message)
        send_confirm(return_route, state)
        {:ok, state}

      {:error, err} ->
        Logger.error("Error unwrapping message: #{inspect(err)}")
        {:error, err}
    end
  end

  def send_confirm(return_route, state) do
    Router.route(%{
      onward_route: return_route,
      return_route: [state.address],
      ## TODO: see `Sender.is_confirm?`
      payload: ""
    })
  end
end
