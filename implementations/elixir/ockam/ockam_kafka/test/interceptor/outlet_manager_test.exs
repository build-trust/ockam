defmodule Ockam.Kafka.Interceptor.OutletManager.Test do
  use ExUnit.Case

  alias Ockam.Kafka.Interceptor.InletManager
  alias Ockam.Kafka.Interceptor.OutletManager
  alias Ockam.Kafka.Interceptor.OutletManager.Outlet

  test "set outlets" do
    outlet_prefix = "outlet_"
    ssl = false
    ssl_options = []

    start_supervised!({OutletManager, [outlet_prefix, ssl, ssl_options]})

    [] = OutletManager.get_existing_outlets(outlet_prefix)
    [] = OutletManager.get_outlets()

    to_create =
      [
        %Outlet{
          outlet_prefix: outlet_prefix,
          node_id: "1",
          target_host: "example.com",
          target_port: 1000
        },
        %Outlet{
          outlet_prefix: outlet_prefix,
          node_id: "2",
          target_host: "example.com",
          target_port: 1001
        }
      ]
      |> Enum.sort()

    OutletManager.set_outlets(to_create)

    assert ^to_create = OutletManager.get_existing_outlets(outlet_prefix) |> Enum.sort()
  end

  test "ssl outlet" do
    outlet_prefix = "outlet_"
    ssl = true
    ssl_options = []

    start_supervised!({OutletManager, [outlet_prefix, ssl, ssl_options]})

    to_create = [
      %Outlet{
        outlet_prefix: outlet_prefix,
        node_id: "1",
        target_host: "example.com",
        target_port: 1000
      }
    ]

    OutletManager.set_outlets(to_create)

    assert ^ssl =
             Ockam.Node.whereis("outlet_1")
             |> :sys.get_state()
             |> Map.get(:worker_options)
             |> Keyword.get(:ssl)
  end

  test "inlet outlet pair" do
    outlet_prefix = "outlet_"
    ssl = false
    ssl_options = []

    base_port = 7000
    allowed_ports = 10
    base_route = []

    start_supervised!({InletManager, [base_port, allowed_ports, base_route, outlet_prefix]})
    start_supervised!({OutletManager, [outlet_prefix, ssl, ssl_options]})

    Ockam.Transport.TCP.start(listen: [port: 4000])

    InletManager.set_inlets(InletManager, [1])

    ## Configure outlet to connect to ockam port 4000
    OutletManager.set_outlets(OutletManager, [
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
  end
end
