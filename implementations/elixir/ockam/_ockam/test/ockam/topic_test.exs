defmodule Ockam.Topic.Tests do
  use ExUnit.Case, async: true
  import ExUnit.CaptureLog
  doctest Ockam.Topic
  alias Ockam.Topic

  @topic_name :test_topic
  @other_topic_name :test_topic_2

  defmodule Consumer do
    @moduledoc """
    Example consumer for Ockam.Topic
    """
    use GenServer
    require Logger

    def init(_state) do
      {:ok, %{}}
    end

    def start_link(default) when is_list(default) do
      GenServer.start_link(__MODULE__, default)
    end

    def destroy(name \\ __MODULE__) do
      GenServer.stop(name)
    end

    def handle_cast({:consume, message}, state) do
      Logger.debug("Should process message: #{message}")
      {:noreply, state}
    end
  end

  setup_all do
    Ockam.Topics.start_link([])
    :ok
  end

  describe "topic" do
    test "create/1" do
      assert {:ok, pid} = Topic.create(@topic_name)
      assert is_pid(pid)
      assert Process.whereis(@topic_name) == pid
      assert Topic.destroy(@topic_name)
    end

    test "can create two topics" do
      assert {:ok, pid} = Topic.create(@topic_name)
      assert is_pid(pid)
      assert Process.whereis(@topic_name) == pid
      assert {:ok, pid} = Topic.create(@other_topic_name)
      assert is_pid(pid)
      assert Process.whereis(@other_topic_name) == pid
      assert Topic.destroy(@topic_name)
      assert Topic.destroy(@other_topic_name)
    end

    test "cannot create twice" do
      {:ok, pid} = Topic.create(@topic_name)
      assert Topic.create(@topic_name) == {:error, {:already_started, pid}}
      assert Topic.destroy(@topic_name) == :ok
    end

    test "destroy/1" do
      Topic.create(@topic_name)
      assert Topic.destroy(@topic_name) == :ok
      assert Process.whereis(@topic_name) == nil
    end

    test "publish/2 with no consumer" do
      Topic.create(@topic_name)
      message = "no consumers"
      assert Topic.publish(@topic_name, message) == :ok
      assert Topic.get_queue(@topic_name) == [message]
      assert Topic.queue_length(@topic_name) == 1
      assert Topic.destroy(@topic_name) == :ok
    end

    test "publish/2 with a consumer" do
      Topic.create(@topic_name)
      message = "1 consumer"
      {:ok, pid} = Consumer.start_link([])
      assert Topic.subscribe(@topic_name, pid) == :ok
      assert Topic.publish(@topic_name, message) == :ok
      # if there is at least one consumer it won't save in the queue
      assert Topic.get_queue(@topic_name) == []
      assert Topic.queue_length(@topic_name) == 0
      assert Topic.destroy(@topic_name) == :ok
    end

    test "subscribe/2" do
      Topic.create(@topic_name)
      {:ok, consumer_pid} = Consumer.start_link([])

      assert Topic.subscribe(@topic_name, consumer_pid) == :ok
      assert Topic.destroy(@topic_name) == :ok
    end

    test "unsubscribe/2" do
      Topic.create(@topic_name)
      {:ok, consumer_pid} = Consumer.start_link([])

      assert Topic.unsubscribe(@topic_name, consumer_pid) == :ok
      assert Topic.destroy(@topic_name) == :ok
    end

    test "process_queued_messages/1" do
      Topic.create(@topic_name)
      {:ok, consumer_pid} = Consumer.start_link([])
      message = "queued message"

      assert Topic.publish(@topic_name, message) == :ok
      assert Topic.get_queue(@topic_name) == [message]
      assert Topic.queue_length(@topic_name) == 1

      # now subscribe a consumer and check that the message is gone
      assert Topic.subscribe(@topic_name, consumer_pid) == :ok
      assert Topic.process_queued_messages(@topic_name) == :ok
      assert Topic.get_queue(@topic_name) == []
      assert Topic.queue_length(@topic_name) == 0

      assert Topic.process_queued_messages(@topic_name) == :ok
      assert Topic.destroy(@topic_name) == :ok
    end
  end

  describe "consumer" do
    test "start_link/1" do
      assert {:ok, pid} = Consumer.start_link([])
      assert Consumer.destroy(pid) == :ok
    end

    test "destroy/1" do
      assert {:ok, pid} = Consumer.start_link([])
      assert Consumer.destroy(pid) == :ok
    end

    test "consume a message" do
      message = "definitely consuming"

      assert capture_log(fn ->
               assert Consumer.handle_cast({:consume, message}, %{}) == {:noreply, %{}}
             end) =~ "Should process message: definitely consuming"
    end
  end
end
