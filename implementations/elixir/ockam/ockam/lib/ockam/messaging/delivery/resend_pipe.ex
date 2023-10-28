defmodule Ockam.Messaging.Delivery.ResendPipe do
  @moduledoc """
  Reliable delivery pipe in which a sender will resend
  the message if it was not confirmed by the receiver
  """

  @behaviour Ockam.Messaging.Pipe

  @impl true
  def sender() do
    __MODULE__.Sender
  end

  @impl true
  def receiver() do
    Ockam.Messaging.ConfirmPipe.Receiver
  end
end

defmodule Ockam.Messaging.Delivery.ResendPipe.Sender do
  @moduledoc """
  Resend pipe sender

  Forwards messages to receiver with a send ref and waits for each message
  to be conformed before sending a next one.
  If a message is not confirmed within confirm_timeout - it's resent with a new ref.

  Options:

  `receiver_route` - a route to receiver
  `confirm_timeout` - time to wait for confirm, default is 5_000
  """
  use Ockam.AsymmetricWorker

  alias Ockam.Messaging.ConfirmPipe.Wrapper

  require Logger

  @default_confirm_timeout 5_000

  @impl true
  def inner_setup(options, state) do
    receiver_route = Keyword.get(options, :receiver_route)
    confirm_timeout = Keyword.get(options, :confirm_timeout, @default_confirm_timeout)

    {:ok,
     Map.merge(state, %{
       receiver_route: receiver_route,
       queue: [],
       confirm_timer: nil,
       confirm_timeout: confirm_timeout
     })}
  end

  ## TODO: batch send
  @impl true
  def handle_outer_message(message, state) do
    case waiting_confirm?(state) do
      true -> enqueue_message(message, state)
      false -> forward_to_receiver(message, state)
    end
  end

  @impl true
  def handle_inner_message(message, state) do
    case is_valid_confirm?(message, state) do
      true ->
        confirm(state)

      false ->
        ## Ignore unknown confirms
        {:ok, state}
    end
  end

  @impl true
  def handle_info(:confirm_timeout, state) do
    with {:ok, state} <- resend_unconfirmed(state) do
      {:noreply, state}
    end
  end

  def resend_unconfirmed(state) do
    ## TODO: do we want to resend with an old ref instead?
    case Map.get(state, :unconfirmed) do
      nil ->
        {:stop, :cannot_resend_unconfirmed, state}

      message ->
        clear_confirm_timeout(state)
        forward_to_receiver(message, state)
    end
  end

  def forward_to_receiver(message, state) do
    forwarded_message = Message.forward(message)

    {ref, state} = bump_send_ref(state)
    {:ok, wrapped_message} = Wrapper.wrap_message(forwarded_message, ref)

    receiver_route = Map.get(state, :receiver_route)

    Ockam.Worker.route(
      %{
        onward_route: receiver_route,
        return_route: [state.inner_address],
        payload: wrapped_message
      },
      state
    )

    {:ok, set_confirm_timeout(message, state)}
  end

  def bump_send_ref(state) do
    ref = Map.get(state, :send_ref, 0) + 1
    {ref, Map.put(state, :send_ref, ref)}
  end

  def set_confirm_timeout(message, state) do
    timeout = Map.get(state, :confirm_timeout)
    timer_ref = Process.send_after(self(), :confirm_timeout, timeout)

    state
    |> Map.put(:confirm_timer, timer_ref)
    |> Map.put(:unconfirmed, message)
  end

  def clear_confirm_timeout(state) do
    case Map.get(state, :confirm_timer) do
      nil ->
        state

      ref ->
        Process.cancel_timer(ref)
        ## Flush the timeout message if it's already received
        receive do
          :confirm_timeout -> :ok
        after
          0 -> :ok
        end

        state
        |> Map.put(:confirm_timer, nil)
        |> Map.put(:unconfirmed, nil)
    end
  end

  def waiting_confirm?(state) do
    ## TODO: should we use confirm_timer or unconfirmed?
    case Map.get(state, :confirm_timer, nil) do
      nil -> false
      _timer -> true
    end
  end

  def is_valid_confirm?(message, state) do
    payload = Message.payload(message)
    {:ok, ref, ""} = :bare.decode(payload, :uint)

    case Map.get(state, :send_ref) do
      current_ref when current_ref == ref ->
        true

      other_ref ->
        Logger.warning(
          "Received confirm for ref #{inspect(ref)}, current ref is #{inspect(other_ref)}"
        )

        false
    end
  end

  def enqueue_message(message, state) do
    queue = Map.get(state, :queue, [])
    {:ok, Map.put(state, :queue, queue ++ [message])}
  end

  def confirm(state) do
    queue = Map.get(state, :queue, [])

    state = clear_confirm_timeout(state)

    case queue do
      [message | rest] ->
        forward_to_receiver(message, Map.put(state, :queue, rest))

      [] ->
        {:ok, state}
    end
  end
end
