defmodule Ockam.Worker.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Worker

  defmodule CapturedFuncion do
    def send(message), do: Kernel.send(:ockam_worker_handler_captured_function_test, message)
  end

  describe "Ockam.Worker.create/1" do
    test "handler can be an anonumous function" do
      caller = self()

      {:ok, worker} = Ockam.Worker.create(fn message -> send(caller, message) end)
      Ockam.Worker.send(worker, "hello")

      assert_receive "hello"

      Ockam.Worker.destroy(worker)
    end

    test "handler can be a captured function" do
      name = :ockam_worker_handler_captured_function_test
      Process.register(self(), name)

      {:ok, worker} = Ockam.Worker.create(&CapturedFuncion.send/1)
      Ockam.Worker.send(worker, "hello")

      assert_receive "hello"
      Process.unregister(name)

      Ockam.Worker.destroy(worker)
    end

    test "fails if handler arity is not 1" do
      assert {:error, _} = Ockam.Worker.create(fn -> IO.puts(100) end)
      assert {:error, _} = Ockam.Worker.create(fn a, b -> IO.puts(a, b) end)
    end

    test "accepts options" do
      Ockam.Worker.create(address: "test", handler: {:stateless, &IO.puts/1})
    end

    test "address can be any term" do
      {:ok, w} = Ockam.Worker.create(address: {:test, 100}, handler: {:stateless, &IO.puts/1})
      assert w == %Ockam.Worker{address: {:test, 100}}
    end
  end

  describe "Ockam.Worker.create/2" do
    test "worker can be stateful" do
      caller = self()

      {:ok, worker} =
        Ockam.Worker.create(100, fn message, state ->
          state = state + message
          send(caller, state)
          {:ok, state}
        end)

      Ockam.Worker.send(worker, 10)
      assert_receive 110

      Ockam.Worker.send(worker, 30)
      assert_receive 140

      Ockam.Worker.destroy(worker)
    end

    test "fails if handler arity is not 2" do
      assert {:error, _} = Ockam.Worker.create(10, fn -> IO.puts(100) end)
      assert {:error, _} = Ockam.Worker.create(10, fn a -> IO.puts(a) end)
    end
  end

  describe "Ockam.Worker.list/0" do
    test "returns list of workers" do
      {:ok, w1} = Ockam.Worker.create(&IO.puts/1)
      {:ok, w2} = Ockam.Worker.create(&IO.puts/1)

      workers = Ockam.Worker.list()
      assert Enum.member?(workers, w1)
      assert Enum.member?(workers, w2)

      Ockam.Worker.destroy(w1)
      Ockam.Worker.destroy(w2)
    end
  end

  describe "Ockam.Worker" do
    test "crashes if handler returns error" do
      {:ok, worker} = Ockam.Worker.create(100, fn _message, _state -> {:error, :invalid} end)
      worker_pid = Ockam.Worker.whereis(worker)

      Process.flag(:trap_exit, true)
      monitor_ref = Process.monitor(worker_pid)

      Ockam.Worker.send(worker, 10)
      assert_receive {:DOWN, _, :process, worker_pid, {:error, :invalid}}

      Process.demonitor(monitor_ref, [:flush])
    end
  end
end
