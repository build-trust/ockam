defmodule Ockam.Topics.Stream.Storage do
  @type message() :: %{message: term(), index: integer()}
  @type topic() :: atom()

  @callback create_topic(topic()) :: {:ok, atom()} | {:error, term()}
  @callback destroy_topic(topic()) :: :ok | {:error, term()}

  @callback push_message(topic(), term()) :: :ok | {:error, term()}
  @callback get_all_messages(topic()) :: {:ok, [message()]} | {:error, term()}
  @callback queue_length(topic()) :: {:ok, integer()} | {:error, term()}
  @callback get_messages(topic(), integer(), integer()) :: {:ok, [message()]} | {:error, term}
  @callback cleanup(topic(), integer()) :: :ok | {:error, term}
end

defmodule Ockam.Topics.Stream.ConsumerStorage do
  @type consumer_id() :: term()
  @type topic() :: atom()
  @type mode() :: :earliest | :latest

  @callback subscribe(topic(), consumer_id(), mode()) :: {:ok, integer()} | {:error, term()}
  @callback get_index(topic(), consumer_id()) :: {:ok, integer()} | {:error, term()}
  @callback confirm(topic(), consumer_id(), integer()) :: {:ok, integer()} | {:error, term()}
  @callback unsubscribe(topic(), consumer_id()) :: :ok | {:error, term()}
end

defmodule Ockam.Topics.Stream.Storage.Memory do
  @behaviour Ockam.Topics.Stream.Storage
  @behaviour Ockam.Topics.Stream.ConsumerStorage

  def create_topic(topic_name) do
    case find_topic(topic_name) do
      {:ok, server} ->
        {:error, {:already_started, server}}

      _ ->
        start_topic(topic_name)
    end
  end

  def start_topic(_topic_name) do
    ## TODO: supervised start
    # Ockam.Topics.Stream.Storage.Memory.Server.start_link()
  end

  def destroy_topic(topic_name) do
    with_topic(topic_name, fn server ->
      GenServer.stop(server)
    end)
  end

  def push_message(topic_name, message) do
    with_topic(topic_name, fn server ->
      GenServer.cast(server, {:push_message, message})
    end)
  end

  def get_all_messages(topic_name) do
    remote_rpc(topic_name, :do_get_all_messages, [topic_name])
  end

  def queue_length(topic_name) do
    remote_rpc(topic_name, :do_get_queue_length, [topic_name])
  end

  def get_messages(topic_name, index, limit) do
    remote_rpc(topic_name, :do_get_messages, [topic_name, index, limit])
  end

  def cleanup(topic_name, index) do
    remote_rpc(topic_name, :do_cleanup, [topic_name, index])
  end

  def with_topic(topic_name, fun) do
    case find_topic(topic_name) do
      {:ok, server} -> fun.(server)
      error -> error
    end
  end

  def find_topic(topic_name) do
    GenServer.whereis(Ockam.Topics.Stream.Storage.Memory.Server.via(topic_name))
  end

  def remote_rpc(topic_name, fun, args) do
    with_topic(topic_name, fn server ->
      case :rpc.call(node(server), __MODULE__, fun, args) do
        {:badrpc, reason} ->
          {:error, {:badrpc, reason}}

        value ->
          {:ok, value}
      end
    end)
  end

  def do_get_all_messages(topic_name) do
    :ets.tab2list(topic_name)
    |> Enum.map(&format_message/1)
  end

  def format_message({index, message}) do
    %{index: index, message: message}
  end

  def do_get_queue_length(topic_name) do
    :ets.info(topic_name, :size)
  end

  def do_get_messages(topic_name, index, limit) do
    :lists.seq(index, index + limit - 1)
    |> Enum.map(fn i -> :ets.lookup(topic_name, i) end)
    |> List.flatten()
    |> Enum.map(&format_message/1)
  end

  def do_cleanup(topic_name, index) do
    [{:earliest, earliest}] = :ets.lookup(topic_name, :earliest)
    do_cleanup(topic_name, earliest, index)
  end

  def do_cleanup(topic_name, from, to) when from < to do
    :lists.seq(from, to)
    |> Enum.each(fn i -> :ets.delete(topic_name, i) end)

    update_earliest(topic_name, to)
  end

  def do_cleanup(_, _, _) do
    :ok
  end

  def update_earliest(topic_name, index) do
    {:ok, server} = find_topic(topic_name)
    GenServer.cast(server, {:update_earliest, index})
  end

  def subscribe(topic_name, consumer_id, mode) do
    with_topic(topic_name, fn server ->
      GenServer.call(server, {:subscribe, consumer_id, mode})
    end)
  end

  def get_index(topic_name, consumer_id) do
    with_topic(topic_name, fn server ->
      GenServer.call(server, {:get_index, consumer_id})
    end)
  end

  def confirm(topic_name, consumer_id, index) do
    with_topic(topic_name, fn server ->
      GenServer.cast(server, {:confirm, consumer_id, index})
    end)
  end

  def unsubscribe(topic_name, consumer_id) do
    with_topic(topic_name, fn server ->
      GenServer.call(server, {:unsubscribe, consumer_id})
    end)
  end
end

defmodule Ockam.Topics.Stream.Storage.Memory.Server do
  use GenServer

  def start_link(topic_name) do
    GenServer.start_link(__MODULE__, topic_name, name: via(topic_name))
  end

  def via(topic_name) do
    {:global, topic_name}
  end

  def init(topic_name) do
    :ets.new(topic_name, [:named_table, :public])
    :ets.insert(topic_name, {:earliest, 0})
    :ets.insert(topic_name, {:latest, 0})
    consumers = :ets.new(:consumers, [:public])
    %{consumers: consumers, topic: topic_name}
  end

  def handle_cast({:push_message, message}, %{topic: topic} = state) do
    [{:latest, index}] = :ets.lookup(topic, :latest)
    next = index + 1
    :ets.insert(topic, {next, message})
    :ets.insert(topic, {:latest, next})
    {:noreply, state}
  end

  def handle_cast({:update_earliest, index}, %{topic: topic} = state) do
    [{:earliest, old_index}] = :ets.lookup(topic, :earliest)
    :ets.insert(topic, {:earliest, max(index, old_index)})
    {:noreply, state}
  end

  def handle_cast({:confirm, consumer_id, index}, %{consumers: consumers} = state) do
    [{consumer_id, old_index}] = :ets.lookup(consumers, consumer_id)

    new_index =
      case old_index do
        i when is_integer(i) -> max(i, index)
        ## Earliest or latest
        _ -> index
      end

    :ets.insert(consumers, {consumer_id, new_index})
    {:noreply, state}
  end

  def handle_call(
        {:subscribe, consumer_id, mode},
        _from,
        %{consumers: consumers, topic: topic} = state
      ) do
    result =
      case :ets.lookup(consumers, consumer_id) do
        [{_consumer_id, _index}] ->
          {:error, :consumer_already_exists}

        [] ->
          :ets.insert(consumers, {consumer_id, mode})
          do_get_index(topic, mode)
      end

    {:reply, result, state}
  end

  def handle_call({:unsubscribe, consumer_id}, _from, %{consumers: consumers} = state) do
    :ets.delete(consumers, consumer_id)
    {:reply, :ok, state}
  end

  def handle_call({:get_index, consumer_id}, _from, %{consumers: consumers, topic: topic} = state) do
    result =
      case :ets.lookup(consumers, consumer_id) do
        [{_consumer_id, index}] ->
          do_get_index(topic, index)

        [] ->
          {:error, :consumer_does_not_exist}
      end

    {:reply, result, state}
  end

  def do_get_index(_topic, index) when is_integer(index) do
    index
  end

  def do_get_index(topic, mode) when is_atom(mode) do
    [{_mode, index}] = :ets.lookup(topic, mode)
    index
  end
end

defmodule Ockam.Topics.Stream.Topic do
  @default_storage Ockam.Topics.Stream.Storage.Memory

  def create(topic_name, storage \\ @default_storage)

  def create(topic_name, storage) when is_atom(topic_name) do
    storage.create_topic(topic_name)
  end

  def destroy(topic_name, storage \\ @default_storage)

  def destroy(topic_name, storage) when is_atom(topic_name) do
    storage.destroy_topic(topic_name)
  end

  def publish(topic_name, message, storage \\ @default_storage)

  def publish(topic_name, message, storage) do
    storage.push_message(topic_name, message, storage)
  end

  def get_queue(topic_name, storage \\ @default_storage)

  def get_queue(topic_name, storage) do
    storage.get_all_messages(topic_name)
  end

  def queue_length(topic_name, storage \\ @default_storage)

  def queue_length(topic_name, storage) do
    storage.queue_length(topic_name)
  end

  ## Caveat: if the earliest index is higher than index+limit,
  ## the limit of messages will be returned starting on the earliest
  def get_messages(topic_name, index, limit, storage \\ @default_storage)

  def get_messages(topic_name, index, limit, storage) do
    storage.get_messages(topic_name, index, limit)
  end

  def cleanup(topic_name, index, storage \\ @default_storage)

  def cleanup(topic_name, index, storage) do
    storage.cleanup(topic_name, index)
  end

  ## Consumer management

  @doc "Crate a new consumer. Returns an error if consumer exists"
  def subscribe(topic_name, consumer_id, mode, consumer_storage \\ @default_storage)

  def subscribe(topic_name, consumer_id, mode, consumer_storage)
      when mode == :earliest or mode == :latest do
    consumer_storage.subscribe(topic_name, consumer_id, mode)
  end

  @doc "Get an existing consumer index. Returns an error if consumer does not exist"
  def get_index(topic_name, consumer_id, consumer_storage \\ @default_storage)

  def get_index(topic_name, consumer_id, consumer_storage) do
    consumer_storage.get_index(topic_name, consumer_id)
  end

  def confirm(topic_name, consumer_id, index, consumer_storage \\ @default_storage)

  def confirm(topic_name, consumer_id, index, consumer_storage) do
    consumer_storage.confirm(topic_name, consumer_id, index)
  end

  def unsubscribe(topic_name, consumer_id, consumer_storage \\ @default_storage) do
    consumer_storage.unsubscribe(topic_name, consumer_id)
  end
end
