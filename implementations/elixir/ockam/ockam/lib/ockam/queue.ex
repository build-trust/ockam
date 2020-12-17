defmodule Ockam.Queue do
  @moduledoc false

  use GenServer

  def create(queue_id) do
    GenServer.start_link(__MODULE__, [], name: name(queue_id))
  end

  def enqueue(queue_id, element) do
    GenServer.call(name(queue_id), {:enqueue, element})
  end

  def dequeue(queue_id, count) do
    GenServer.call(name(queue_id), {:dequeue, count})
  end

  def read(queue_id, starting_index, count) do
    GenServer.call(name(queue_id), {:read, starting_index, count})
  end

  def get_length(queue_id) do
    GenServer.call(name(queue_id), :length)
  end

  def init([]) do
    {:ok, %{elements: []}}
  end

  def handle_call({:enqueue, element}, _, state) do
    {:reply, :ok, %{state | elements: state.elements ++ [element]}}
  end

  def handle_call({:read, starting_index, count}, _, state) do
    elements =
      state.elements
      |> Enum.drop(starting_index)
      |> Enum.take(count)

    {:reply, elements, state}
  end

  def handle_call({:dequeue, count}, _, state) do
    {return, rest} = Enum.split(state.elements, count)

    {:reply, return, %{state | elements: rest}}
  end

  def handle_call(:length, _, state) do
    {:reply, length(state.elements), state}
  end

  def name(queue_id) do
    {:via, Registry, {:queue_registry, queue_id}}
  end
end
