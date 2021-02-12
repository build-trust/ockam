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

    def init(topic_name) do
      {:ok, %{last_index: 0, topic_name: topic_name}}
    end

    def start_link(topic_name) do
      GenServer.start_link(__MODULE__, topic_name)
    end

    def destroy(name \\ __MODULE__) do
      GenServer.stop(name)
    end

    def handle_cast({:consume, message}, state) do
      Logger.debug("Should process message: #{message}")

      {:noreply, state}
    end

    def handle_cast({:consume, message, index}, state) do
      Logger.debug("Should process message: #{message} with index #{index}")
      {:noreply, %{state | last_index: index}}
    end

    def handle_call({:confirm, index}, _from, %{last_index: last_index, topic_name: topic_name} = state) do
      if index <= last_index do
        Topic.confirm(topic_name, self(), index)
        {:reply, {:ok, index}, state}
      else
        {:reply, {:last_index, last_index}, state}
      end
    end

    def handle_call(:last_index, _from, %{last_index: last_index} = state) do
      {:reply, last_index, state}
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
      {:ok, pid} = Consumer.start_link(@topic_name)
      assert Topic.subscribe(@topic_name, pid) == :ok
      assert Topic.publish(@topic_name, message) == :ok

      wait_for_received(pid, 1)
      GenServer.call(pid, {:confirm, 1})

      assert Topic.get_queue(@topic_name) == []
      assert Topic.queue_length(@topic_name) == 0
      assert Topic.destroy(@topic_name) == :ok
    end

    test "publish/2 with a consumer limit" do
      Topic.create(@topic_name)
      message = "1 consumer"
      {:ok, pid} = Consumer.start_link(@topic_name)
      assert Topic.subscribe(@topic_name, pid) == :ok

      :lists.seq(1, 20)
      |> Enum.each(fn(i) -> Topic.publish(@topic_name, "#{i} message") end)

      assert {:ok, 10} == wait_for_received(pid, 10)

      ## Don't receive more
      Topic.publish(@topic_name, "21 message")

      assert {:error, {:low_index, 10}} == wait_for_received(pid, 11, 1000)

      assert Topic.queue_length(@topic_name) == 21

      GenServer.call(pid, {:confirm, 10})

      assert {:ok, 20} == wait_for_received(pid, 20)

      assert Topic.queue_length(@topic_name) == 11

      GenServer.call(pid, {:confirm, 20})
      GenServer.call(pid, {:confirm, 21})

      assert Topic.get_queue(@topic_name) == []
      assert Topic.queue_length(@topic_name) == 0
      assert Topic.destroy(@topic_name) == :ok
    end

    test "subscribe/2" do
      Topic.create(@topic_name)
      {:ok, consumer_pid} = Consumer.start_link(@topic_name)

      assert Topic.subscribe(@topic_name, consumer_pid) == :ok
      assert Topic.destroy(@topic_name) == :ok
    end

    test "unsubscribe/2" do
      Topic.create(@topic_name)
      {:ok, consumer_pid} = Consumer.start_link(@topic_name)

      assert Topic.unsubscribe(@topic_name, consumer_pid) == :ok
      assert Topic.destroy(@topic_name) == :ok
    end
  end

  def wait_for_received(pid, index, timeout \\ 5000)
  def wait_for_received(pid, index, timeout) do
    wait_for(fn() ->
      case GenServer.call(pid, :last_index) do
        i when i >= index -> {:ok, i}
        i -> {:error, {:low_index, i}}
      end
    end, timeout)
  end

  def wait_for(fun, timeout, error \\ nil)
  def wait_for(fun, timeout, error) when timeout <= 0 do
    error
  end
  def wait_for(fun, timeout, error) do
    case fun.() do
      {:ok, result} -> {:ok, result}
      {:error, _} = error ->
        :timer.sleep(100)
        wait_for(fun, timeout - 100, error)
    end
  end

  describe "consumer" do
    test "start_link/1" do
      assert {:ok, pid} = Consumer.start_link(@topic_name)
      assert Consumer.destroy(pid) == :ok
    end

    test "destroy/1" do
      assert {:ok, pid} = Consumer.start_link(@topic_name)
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
