defmodule Ockam.QueueTest do
  use ExUnit.Case

  alias Ockam.Queue

  setup do
    {:ok, _} = Registry.start_link(keys: :unique, name: :queue_registry)

    :ok
  end

  test "items can be added and read from the queue" do
    queue_id = random_id()
    {:ok, _pid} = Queue.create(queue_id)

    :ok = Queue.enqueue(queue_id, 10)
    :ok = Queue.enqueue(queue_id, 11)
    :ok = Queue.enqueue(queue_id, 12)

    assert Queue.read(queue_id, 0, 1) == [10]
    assert Queue.read(queue_id, 1, 2) == [11, 12]
    assert Queue.read(queue_id, 2, 5) == [12]
  end

  test "you can fetch the length of the queue" do
    queue_id = random_id()
    {:ok, _pid} = Queue.create(queue_id)

    :ok = Queue.enqueue(queue_id, 10)
    :ok = Queue.enqueue(queue_id, 11)
    :ok = Queue.enqueue(queue_id, 12)

    assert Queue.get_length(queue_id) == 3
  end

  test "elements can be dequeued from the top" do
    queue_id = random_id()
    {:ok, _pid} = Queue.create(queue_id)

    :ok = Queue.enqueue(queue_id, 10)
    :ok = Queue.enqueue(queue_id, 11)
    :ok = Queue.enqueue(queue_id, 12)

    assert [10, 11] = Queue.dequeue(queue_id, 2)
    assert Queue.get_length(queue_id) == 1
  end

  test "queues can be created dynamically" do
    queue1 = random_id()
    queue2 = random_id()
    {:ok, _pid} = Queue.create(queue1)
    {:ok, _pid} = Queue.create(queue2)

    :ok = Queue.enqueue(queue1, 10)
    :ok = Queue.enqueue(queue2, 20)

    assert Queue.read(queue1, 0, 1) == [10]
    assert Queue.read(queue2, 0, 1) == [20]
  end

  defp random_id do
    "queue-#{:random.uniform(1_000_000)}"
  end
end
