defmodule Ockam.Worker.Tests.InnerWorker do
  use Ockam.Worker

  @impl true
  def handle_message(_message, state) do
    {:ok, state}
  end
end

defmodule Ockam.Worker.Tests.OuterWorker do
  use Ockam.Worker

  @impl true
  def setup(options, state) do
    {:ok, inner_worker} = Ockam.Worker.Tests.InnerWorker.create([])
    {:ok, Map.put(state, :inner, inner_worker)}
  end

  @impl true
  def handle_message(_message, state) do
    {:ok, state}
  end

  @impl true
  def handle_call(:get_inner, _from, state) do
    {:reply, Map.get(state, :inner), state}
  end
end

defmodule Ockam.Worker.Tests do
  use ExUnit.Case, async: false
  doctest Ockam.Worker

  require Logger

  describe "Ockam.Worker" do
    test "Can start inner worker on setup" do
      {:ok, address} = Ockam.Worker.Tests.OuterWorker.create([])
      pid = Ockam.Node.whereis(address)
      assert is_binary(GenServer.call(pid, :get_inner))
    end
  end
end
