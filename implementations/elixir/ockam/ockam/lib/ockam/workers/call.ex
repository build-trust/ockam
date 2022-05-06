defmodule Ockam.Workers.Call do
  @moduledoc """
  One-off worker to perform a synchronous call to an Ockam worker using
  the routing protocol.
  """

  use Ockam.Worker

  require Logger

  def call_on_current_process(payload, onward_route, timeout \\ 10_000, self_address \\ nil) do
    with_self_address(self_address, fn call_address ->
      Ockam.Router.route(payload, onward_route, [call_address])

      receive do
        %Ockam.Message{
          onward_route: [^call_address]
        } = message ->
          {:ok, message}
      after
        timeout ->
          {:error, :timeout}
      end
    end)
  end

  def with_self_address(nil, fun) do
    {:ok, call_address} = Ockam.Node.register_random_address()

    try do
      fun.(call_address)
    after
      Ockam.Node.unregister_address(call_address)
      ## Flush the mailbox
      receive do
        %Ockam.Message{onward_route: [^call_address]} -> :ok
      after
        0 -> :ok
      end
    end
  end

  def with_self_address(address, fun) do
    fun.(address)
  end

  def call(call, options \\ [], timeout \\ 20_000) do
    {:ok, address} = __MODULE__.create(Keyword.merge(options, call: call))

    [message] = GenServer.call(Ockam.Node.whereis(address), :fetch, timeout)
    message
  end

  def call_multi(call, reply_number, options \\ [], timeout \\ 20_000) do
    {:ok, address} = __MODULE__.create(Keyword.put(options, :call, call))
    GenServer.call(Ockam.Node.whereis(address), {:fetch_multi, reply_number}, timeout)
  end

  @impl true
  def address_prefix(_options), do: "CALL_"

  @impl true
  def setup(options, %{} = state) do
    call = Keyword.fetch!(options, :call)
    send_call(call, state)
    {:ok, state}
  end

  def send_call(call, state) do
    Ockam.Router.route(%{
      payload: Map.get(call, :payload),
      onward_route: Map.get(call, :onward_route),
      return_route: [state.address]
    })
  end

  @impl true
  def handle_message(%{payload: _} = message, state) do
    state = Map.update(state, :messages, [message], fn messages -> messages ++ [message] end)

    maybe_reply(state)
  end

  def maybe_reply(state) do
    case Map.get(state, :wait) do
      nil ->
        {:ok, state}

      {waiter, num_messages} ->
        case Map.get(state, :messages) do
          messages when length(messages) >= num_messages ->
            GenServer.reply(waiter, messages)
            ## Terminate here
            {:stop, :shutdown, state}

          _other ->
            {:ok, state}
        end
    end
  end

  @impl true
  def handle_call(:fetch, from, state) do
    state = Map.put(state, :wait, {from, 1})

    with {:ok, state} <- maybe_reply(state) do
      {:noreply, state}
    end
  end

  def handle_call({:fetch_multi, reply_number}, from, state) do
    state = Map.put(state, :wait, {from, reply_number})

    with {:ok, state} <- maybe_reply(state) do
      {:noreply, state}
    end
  end
end
