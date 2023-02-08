defmodule Ockam.Kafka.Interceptor.InletManager.Test do
  use ExUnit.Case

  alias Ockam.Kafka.Interceptor.InletManager

  test "set inlets" do
    base_port = 8000
    allowed_ports = 10
    base_route = ["outlets"]
    outlet_prefix = "outlet_"

    {:ok, _manager} =
      InletManager.start_link([base_port, allowed_ports, base_route, outlet_prefix])

    on_exit(fn ->
      try do
        GenServer.stop(InletManager)
      catch
        _type, _reason -> :ok
      end
    end)

    inlets = InletManager.list_inlets()

    assert map_size(inlets) == 0

    {:error, :port_out_of_range} = InletManager.set_inlets([20])

    :ok = InletManager.set_inlets([1, 2, 3, 4])

    inlets = InletManager.list_inlets()

    assert map_size(inlets) == 4

    assert Enum.sort(Map.keys(inlets)) == [1, 2, 3, 4]

    :ok = InletManager.set_inlets([1, 3, 5])

    inlets = InletManager.list_inlets()

    assert map_size(inlets) == 3
  end

  test "inlet internals" do
    base_port = 8000
    allowed_ports = 10
    base_route = ["outlets"]
    outlet_prefix = "outlet_"

    {:ok, _manager} =
      InletManager.start_link([base_port, allowed_ports, base_route, outlet_prefix])

    on_exit(fn ->
      try do
        GenServer.stop(InletManager)
      catch
        _type, _reason -> :ok
      end
    end)

    :ok = InletManager.set_inlets([1])
    inlets = InletManager.list_inlets()

    inlet1_state = Map.get(inlets, 1) |> :sys.get_state()

    assert Map.get(inlet1_state, :peer_route) == base_route ++ [outlet_prefix <> "1"]

    expected_port = base_port + 1

    assert {:ok, ^expected_port} = Map.get(inlet1_state, :listen_socket) |> :inet.port()
  end
end
