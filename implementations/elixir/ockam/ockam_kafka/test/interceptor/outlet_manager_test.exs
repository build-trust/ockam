defmodule Ockam.Kafka.Interceptor.OutletManager.Test do
  use ExUnit.Case

  alias Ockam.Kafka.Interceptor.OutletManager
  alias Ockam.Kafka.Interceptor.OutletManager.Outlet

  test "set outlets" do
    outlet_prefix = "outlet_"
    ssl = false
    ssl_options = []

    {:ok, _manager} = start_supervised({OutletManager, [outlet_prefix, ssl, ssl_options]})

    on_exit(fn ->
      try do
        GenServer.stop(OutletManager)
      catch
        _type, _reason -> :ok
      end
    end)

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

    {:ok, _manager} = start_supervised({OutletManager, [outlet_prefix, ssl, ssl_options]})

    on_exit(fn ->
      try do
        GenServer.stop(OutletManager)
      catch
        _type, _reason -> :ok
      end
    end)

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
end
