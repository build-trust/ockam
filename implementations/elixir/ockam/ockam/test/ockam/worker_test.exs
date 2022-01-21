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
  def setup(_options, state) do
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

  alias Ockam.Worker.Tests.InnerWorker
  alias Ockam.Worker.Tests.OuterWorker

  require Logger

  describe "Ockam.Worker" do
    test "Can start inner worker on setup" do
      {:ok, address} = OuterWorker.create([])
      pid = Ockam.Node.whereis(address)
      assert is_binary(GenServer.call(pid, :get_inner))
    end

    test "Can start a worker with address prefix" do
      {:ok, address} = InnerWorker.create(address_prefix: "inner_")
      assert "inner_" <> _name = address
    end

    test "Explicit address overrides address prefix" do
      {:ok, address} = InnerWorker.create(address: "explicit_inner", address_prefix: "inner_")

      assert "explicit_inner" == address
    end
  end
end
