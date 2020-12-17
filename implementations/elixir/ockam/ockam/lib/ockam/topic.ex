defmodule Ockam.Topics do
  @moduledoc """
  Ockam.Topics
  """

  @doc false
  use DynamicSupervisor

  require Logger

  def start_link(init_arg) do
    DynamicSupervisor.start_link(__MODULE__, init_arg, name: __MODULE__)
  end

  @impl true
  def init(_init_arg) do
    DynamicSupervisor.init(strategy: :one_for_one)
  end

  @spec start_child(atom, atom) :: :ignore | {:error, any} | {:ok, pid} | {:ok, pid, any}
  def start_child(module, topic_name) when is_atom(topic_name) do
    DynamicSupervisor.start_child(__MODULE__, %{
      id: topic_name,
      start: {module, :start_link, [topic_name]}
    })
  end

  @spec terminate_child(atom) :: :ok | {:error, :not_found}
  def terminate_child(topic_name) do
    pid = Process.whereis(topic_name)
    DynamicSupervisor.terminate_child(__MODULE__, pid)
  end
end

defmodule Ockam.Topic do
  @moduledoc """
  Implements the publish and subscribe semantics.
  """
  use GenServer

  alias Ockam.Topics

  def init(%{topic_name: _} = state) do
    # @TODO use ets for messages
    # @TODO register consumers with their own DynamicSupervisor
    # @TODO queued_messages is unbounded and really ought to be a pluggable backend with an ets table.
    {:ok, Map.merge(state, %{queued_messages: [], consumers: []})}
  end

  def start_link(topic_name) do
    GenServer.start_link(__MODULE__, %{topic_name: topic_name}, name: topic_name)
  end

  def create(topic_name) when is_atom(topic_name) do
    # @TODO check for existing topic
    Topics.start_child(__MODULE__, topic_name)
  end

  def destroy(topic_name) when is_atom(topic_name) do
    Topics.terminate_child(topic_name)
  end

  def publish(topic_name, message) do
    GenServer.cast(topic_name, {:publish, message})
  end

  def get_queue(topic_name) do
    GenServer.call(topic_name, :get_queue)
  end

  def queue_length(topic_name) do
    GenServer.call(topic_name, :queue_length)
  end

  def subscribe(topic_name, pid) do
    GenServer.call(topic_name, {:subscribe, pid})
  end

  def process_queued_messages(topic_name) do
    GenServer.cast(topic_name, :process_queued_messages)
  end

  def unsubscribe(topic_name, pid) do
    GenServer.call(topic_name, {:unsubscribe, pid})
  end

  # @TODO add a timer for processing the queued messages.

  def handle_cast({:publish, message}, %{queued_messages: messages, consumers: []} = state) do
    # ++ performance does not suck anymore
    {:noreply, %{state | queued_messages: [message | messages]}}
  end

  def handle_cast({:publish, message}, %{consumers: consumers} = state) do
    Enum.each(consumers, fn consumer ->
      # @TODO this should be a protocol probably.
      GenServer.cast(consumer, {:consume, message})
    end)

    {:noreply, state}
  end

  def handle_cast(
        :process_queued_messages,
        %{queued_messages: messages, topic_name: topic_name} = state
      ) do
    messages
    |> Enum.reverse()
    |> Enum.each(fn message ->
      publish(topic_name, message)
    end)

    {:noreply, %{state | queued_messages: []}}
  end

  def handle_call(:get_queue, _from, %{queued_messages: messages} = state) do
    {:reply, messages, state}
  end

  def handle_call(:queue_length, _from, %{queued_messages: messages} = state) do
    {:reply, length(messages), state}
  end

  def handle_call(
        {:subscribe, pid},
        _from,
        %{topic_name: topic_name, consumers: consumers} = state
      ) do
    process_queued_messages(topic_name)
    {:reply, :ok, %{state | consumers: [pid | consumers]}}
  end

  def handle_call({:unsubscribe, pid}, _from, %{consumers: consumers} = state) do
    consumers =
      Enum.reject(consumers, fn consumer ->
        consumer == pid
      end)

    {:reply, :ok, %{state | consumers: consumers}}
  end
end
