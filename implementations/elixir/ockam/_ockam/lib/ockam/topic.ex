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

  require Logger

  @storage_module Ockam.Topics.Log.Map
  @consumers_module Ockam.Topics.Consumers.Map

  def init(%{topic_name: topic_name} = state) do
    # @TODO use ets for messages
    # @TODO register consumers with their own DynamicSupervisor
    # @TODO queued_messages is unbounded and really ought to be a pluggable backend with an ets table.
    storage = {@storage_module, @storage_module.init(topic_name)}
    consumers = {@consumers_module, @consumers_module.init()}
    {:ok, Map.merge(state, %{storage: storage, consumers: consumers})}
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

  def subscribe(topic_name, pid, limit \\ 10) do
    GenServer.call(topic_name, {:subscribe, pid, limit})
  end

  def unsubscribe(topic_name, pid) do
    GenServer.call(topic_name, {:unsubscribe, pid})
  end

  def confirm(topic_name, pid, index) do
    GenServer.cast(topic_name, {:confirm, pid, index})
  end

  # @TODO add a timer for processing the queued messages.

  def handle_cast({:publish, message}, state) do
    new_state =
      state
      |> enqueue(message)
      |> bump_consumers()

    ## TODO: when using distributed storage it might be a challenge
    ## to bump consumers

    ## In that case consumers checking storage periodically would be better

    {:noreply, new_state}
  end

  def handle_cast({:confirm, pid, index}, state) do
    new_state =
      state
      |> confirm_consumer_index(pid, index)
      |> consume_messages(pid)
      |> cleanup_log()
    {:noreply, new_state}
  end

  def handle_call(:get_queue, _from, state) do
    {storage_module, storage} = state.storage
    {:reply, storage_module.get_all_messages(storage), state}
  end

  def handle_call(:queue_length, _from, state) do
    {storage_module, storage} = state.storage
    {:reply, storage_module.get_queue_length(storage), state}
  end

  def handle_call(
        {:subscribe, pid, limit},
        _from,
        state
      ) do
    {s_mod, storage} = state.storage
    earliest = s_mod.get_earliest(storage)
    new_state =
      state
      |> Map.update!(:consumers, fn({mod, mod_state}) -> {mod, mod.add_consumer(mod_state, pid, earliest, limit)} end)
      |> consume_messages(pid)
    {:reply, :ok, new_state}
  end

  def handle_call({:unsubscribe, pid}, _from, state) do
    new_state = state |> Map.update!(:consumers, fn({mod, mod_state}) -> {mod, mod.remove_consumer(mod_state, pid)} end)
    ## TODO: cleanup log
    {:reply, :ok, new_state}
  end

  def enqueue(state, message) do
    {storage_module, storage} = state.storage
    storage = storage_module.store(storage, message)
    %{state | storage: {storage_module, storage}}
  end

  def cleanup_log(state) do
    {s_mod, storage} = state.storage
    {c_mod, consumers} = state.consumers

    min = c_mod.min_confirmed_index(consumers)
    new_storage = s_mod.cleanup(storage, min)
    %{state | storage: {s_mod, new_storage}}
  end

  def bump_consumers(state) do
    {mod, consumers} = state.consumers
    mod.get_consumer_ids(consumers)
    |> Enum.reduce(state, fn(pid, state_acc) -> consume_messages(state_acc, pid) end)
  end

  def consume_messages(state, consumer_pid) do
    {c_mod, consumers} = state.consumers
    {s_mod, storage} = state.storage

    consumer_data = c_mod.get_consumer(consumers, consumer_pid)
    latest = s_mod.get_latest(storage)

    case consumer_data.sent < latest do
      true ->
        limit = consumer_data.limit
        from = consumer_data.sent + 1
        to = min(latest, consumer_data.confirmed + limit)
        s_mod.get_messages(storage, from, to)
        |> Enum.each(fn(message) -> send_message(message, consumer_pid) end)
        update_sent(state, consumer_pid, to)
      false ->
        state
    end
  end

  def send_message({index, message}, consumer_pid) do
    GenServer.cast(consumer_pid, {:consume, message, index})
  end

  def update_sent(state, consumer_pid, index) do
    {mod, consumers} = state.consumers

    consumers = mod.update_consumer(consumers, consumer_pid, :sent, index)
    %{state | consumers: {mod, consumers}}
  end

  def confirm_consumer_index(state, consumer_pid, index) do
    {mod, consumers} = state.consumers

    consumers = mod.update_consumer(consumers, consumer_pid, :confirmed, index)
    %{state | consumers: {mod, consumers}}
  end
end

defmodule Ockam.Topics.Consumers.Map do
  def init() do
    %{}
  end

  def update_consumer(consumers, consumer_pid, key, index) do
    case get_consumer(consumers, consumer_pid) do
      nil -> consumers
      consumer ->
        Map.put(consumers, consumer_pid, Map.put(consumer, key, index))
    end
  end

  def add_consumer(consumers, consumer_pid, init, limit) do
    case Map.get(consumers, consumer_pid) do
      nil ->
        consumers |> Map.put(consumer_pid, %{confirmed: init, sent: init, limit: limit})
      _ ->
        consumers
    end
  end

  def remove_consumer(consumers, consumer_pid) do
    Map.delete(consumers, consumer_pid)
  end

  def get_consumer(consumers, consumer_pid) do
    Map.get(consumers, consumer_pid)
  end

  def min_confirmed_index(consumers) do
    consumers
    |> Map.values
    |> Enum.min_by(fn(%{confirmed: confirmed}) -> confirmed end)
    |> Map.get(:confirmed)
  end

  def get_consumer_ids(consumers) do
    Map.keys(consumers)
  end
end


defmodule Ockam.Topics.Log.Map do
  def init(_topic_name) do
    %{earliest: 0, latest: 0}
  end

  def cleanup(storage, index) do
    earliest = get_earliest(storage)
    to_cleanup = case earliest <= index do
      true -> :lists.seq(earliest, index)
      false -> []
    end

    to_cleanup
    |> Enum.reduce(storage, fn(index, acc) -> Map.delete(acc, index) end)
    |> Map.put(:earliest, index + 1)
  end

  def get_messages(storage, from, to) do
    :lists.seq(from, to)
    |> Enum.map(fn(index) ->
      {index, Map.get(storage, index)}
    end)
    |> Enum.filter(fn({_, nil}) -> false; (_) -> true end)
  end

  def get_earliest(storage) do
    Map.get(storage, :earliest)
  end

  def get_latest(storage) do
    Map.get(storage, :latest)
  end

  def get_queue_length(storage) do
    map_size(storage) - 2
  end

  def get_all_messages(storage) do
    storage
    |> Map.delete(:latest)
    |> Map.delete(:earliest)
    |> Enum.sort_by(fn({i, _m}) -> i end)
    |> Enum.map(fn({_i, m}) -> m end)
  end

  def store(storage, message) do
    latest = Map.get(storage, :latest)
    next = latest + 1
    storage
    |> Map.put(next, message)
    |> Map.put(:latest, next)
  end
end

