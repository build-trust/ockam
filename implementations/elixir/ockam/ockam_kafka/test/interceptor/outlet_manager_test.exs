defmodule Ockam.Kafka.Interceptor.OutletManager.Test.TcpEchoer do
  @behaviour :ranch_protocol

  def start_link(ref, transport, opts) do
    pid = spawn_link(__MODULE__, :init, [ref, transport, opts])
    {:ok, pid}
  end

  def init(ref, transport, _opts) do
    {:ok, socket} = :ranch.handshake(ref)
    loop(socket, transport)
  end

  defp loop(socket, transport) do
    case transport.recv(socket, 0, 5000) do
      {:ok, data} ->
        transport.send(socket, data)
        loop(socket, transport)

      _ ->
        :ok = transport.close(socket)
    end
  end

  def start(port) do
    :ranch.start_listener(:echo_listener, :ranch_tcp, [port: port], __MODULE__, [])
    :ok
  end

  def close() do
    :ranch.stop_listener(:echo_listener)
  end
end

defmodule Ockam.Kafka.Interceptor.OutletManager.Test do
  use ExUnit.Case

  alias Ockam.Kafka.Interceptor.InletManager
  alias Ockam.Kafka.Interceptor.OutletManager
  alias Ockam.Kafka.Interceptor.OutletManager.Outlet
  alias Ockam.Kafka.Interceptor.OutletManager.Test.TcpEchoer

  test "set outlets" do
    outlet_prefix = "outlet_"
    ssl = false
    ssl_options = []

    start_supervised!(
      {OutletManager, [outlet_prefix: outlet_prefix, ssl: ssl, ssl_options: ssl_options]}
    )

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

    start_supervised!(
      {OutletManager, [outlet_prefix: outlet_prefix, ssl: ssl, ssl_options: ssl_options]}
    )

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

    base_port = 11_000
    allowed_ports = 10
    base_route = []

    start_supervised!(
      {InletManager,
       [
         base_port: base_port,
         allowed_ports: allowed_ports,
         base_route: base_route,
         outlet_prefix: outlet_prefix
       ]}
    )

    start_supervised!(
      {OutletManager, [outlet_prefix: outlet_prefix, ssl: ssl, ssl_options: ssl_options]}
    )

    :ok = TcpEchoer.start(12_000)

    InletManager.set_inlets(InletManager, [1])

    ## Configure outlet to connect to ockam port 12_000
    OutletManager.set_outlets(OutletManager, [
      %Outlet{
        outlet_prefix: outlet_prefix,
        node_id: "1",
        target_host: "localhost",
        target_port: 12_000
      }
    ])

    ## Connect to inlet port 11_001
    {:ok, socket} =
      :gen_tcp.connect('localhost', 11_001, [:binary, {:packet, 2}, {:active, false}])

    ## Send message to inlet
    :ok = :gen_tcp.send(socket, "HI")

    ## log socket
    IO.inspect(socket)

    ## Receive message from outlet
    {:ok, data} = :gen_tcp.recv(socket, 0)
    assert "HI" = data

    :ok = :gen_tcp.close(socket)
    TcpEchoer.close()
  end
end
