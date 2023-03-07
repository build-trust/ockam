defmodule Ockam.Kafka.Interceptor.InletOutlet.Test do
  use ExUnit.Case

  alias Ockam.Kafka.Interceptor.InletManager
  alias Ockam.Kafka.Interceptor.OutletManager
  alias Ockam.Kafka.Interceptor.OutletManager.Outlet

  test "inlet outlet pair" do
    outlet_prefix = "outlet_"
    ssl = false
    ssl_options = []

    base_port = 7000
    allowed_ports = 10
    base_route = []

    {:ok, _inlet_manager} =
      InletManager.start_link([base_port, allowed_ports, base_route, outlet_prefix])

    {:ok, _outlet_manager} = OutletManager.start_link([outlet_prefix, ssl, ssl_options])

    on_exit(fn ->
      try do
        GenServer.stop(OutletManager)
      catch
        _type, _reason -> :ok
      end

      try do
        GenServer.stop(InletManager)
      catch
        _type, _reason -> :ok
      end
    end)

    Ockam.Transport.TCP.start(listen: [port: 4000])

    InletManager.set_inlets([1])

    ## Configure outlet to connect to ockam port 4000
    OutletManager.set_outlets([
      %Outlet{
        outlet_prefix: outlet_prefix,
        node_id: "1",
        target_host: "localhost",
        target_port: 4000
      }
    ])

    ## Connect to inlet port
    {:ok, client} = Ockam.Transport.TCP.Client.create(destination: {"localhost", 7001})

    {:ok, "echo"} = Ockam.Services.Echo.create(address: "echo")

    ## We can call "echo" via inlet-outlet pair
    assert {:ok, %Ockam.Message{payload: "HI"}} =
             Ockam.Workers.Call.call_on_current_process("HI", [client, "echo"])

    GenServer.stop(OutletManager)
    GenServer.stop(InletManager)
  end
end
