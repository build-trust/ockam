defmodule Ockam.Kafka.Interceptor.Test.FakeOutlet do
  use Ockam.Worker

  alias Ockam.Kafka.Interceptor.Protocol.Formatter

  alias Ockam.Kafka.Interceptor.Protocol.Metadata.Response, as: MetadataResponse
  alias Ockam.Kafka.Interceptor.Protocol.Metadata.Response.Formatter, as: ResponseFormatter

  alias Ockam.Kafka.Interceptor.Protocol.ResponseHeader

  alias Ockam.Message
  alias Ockam.Worker

  alias Ockam.Transport.Portal.TunnelProtocol

  require Logger

  @api_version 12

  @impl true
  def handle_message(tunnel_message, state) do
    case TunnelProtocol.decode(tunnel_message.payload) do
      {:ok, :ping} ->
        tunnel_message
        |> Message.reply(state.address, TunnelProtocol.encode(:pong))
        |> Worker.route(state)

        {:ok, Map.put(state, :peer_route, Message.return_route(tunnel_message))}

      {:ok, :disconnect} ->
        {:ok, state}

      {:ok, {:payload, data}} ->
        response = make_response(data)

        Worker.route(
          %Message{
            onward_route: Map.get(state, :peer_route),
            payload: TunnelProtocol.encode({:payload, {response, :undefined}})
          },
          state
        )

        {:ok, state}
    end
  end

  def make_response(_data) do
    ## TODO: correlation id parsing
    correlation_id = 1

    response = %MetadataResponse{
      api_version: @api_version,
      throttle_time_ms: 1000,
      brokers: brokers(),
      cluster_id: "cluster1",
      controller_id: 1,
      topics: [],
      tagged_fields: %{}
    }

    {:ok, response_bin} = ResponseFormatter.format(response)

    {:ok, header_bin} =
      Formatter.format_response_header(%ResponseHeader{
        header_version: 1,
        correlation_id: correlation_id,
        tagged_fields: %{}
      })

    data = header_bin <> response_bin
    size = byte_size(data)
    <<size::signed-big-integer-size(32), data::binary>>
  end

  def brokers() do
    [
      %MetadataResponse.Broker{
        node_id: 0,
        host: "example.com",
        port: 123,
        tagged_fields: %{}
      },
      %MetadataResponse.Broker{
        node_id: 1,
        host: "example1.com",
        port: 1234,
        tagged_fields: %{}
      }
    ]
  end
end

defmodule Ockam.Kafka.Interceptor.Test do
  use ExUnit.Case

  alias Ockam.Kafka.Interceptor.MetadataHandler

  alias Ockam.Kafka.Interceptor.Protocol.Formatter

  alias Ockam.Kafka.Interceptor.Protocol.Metadata.Response, as: MetadataResponse

  alias Ockam.Kafka.Interceptor.Protocol.Metadata.Request, as: MetadataRequest
  alias Ockam.Kafka.Interceptor.Protocol.Metadata.Request.Formatter, as: RequestFormatter

  alias Ockam.Kafka.Interceptor.Protocol.Parser

  alias Ockam.Kafka.Interceptor.Protocol.RequestHeader

  alias Ockam.Kafka.Interceptor.Test.FakeOutlet

  alias Ockam.Kafka.Interceptor.InletManager
  alias Ockam.Kafka.Interceptor.OutletManager

  @api_metadata 3
  @api_version 12

  test "intercept kafka requests" do
    {:ok, _pid, outlet} = FakeOutlet.start_link(address: "kafka_bootstrap")

    bootstrap_port = 9000
    base_port = 9001
    allowed_ports = 10
    outlet_route = ["kafka_interceptor"]
    outlet_prefix = "outlet_"
    bootstrap_route = outlet_route ++ [outlet]

    connect_timeout = 10_000

    {:ok, _pid} =
      Ockam.Transport.Portal.InletListener.start_link(
        port: bootstrap_port,
        peer_route: bootstrap_route
      )

    start_supervised!(
      {InletManager,
       [
         base_port: base_port,
         allowed_ports: allowed_ports,
         base_route: outlet_route,
         outlet_prefix: outlet_prefix
       ]}
    )

    start_supervised!(
      {OutletManager, [outlet_prefix: outlet_prefix, ssl: false, ssl_options: []]}
    )

    {:ok, _pid, "kafka_interceptor"} =
      Ockam.Transport.Portal.Interceptor.start_link(
        address: "kafka_interceptor",
        interceptor_mod: Ockam.Kafka.Interceptor,
        interceptor_options: [
          handler_options: [
            base_port: base_port
          ],
          response_handlers: [
            &MetadataHandler.outlet_response/3,
            &MetadataHandler.inlet_response/3
          ]
        ]
      )

    {:ok, sock} =
      :gen_tcp.connect(
        'localhost',
        bootstrap_port,
        [{:active, false}, :binary, {:packet, 0}],
        connect_timeout
      )

    metadata_request = make_metadata_request(1)

    :ok = :gen_tcp.send(sock, metadata_request)

    {:ok, packet} = :gen_tcp.recv(sock, 0)

    assert <<size::signed-big-integer-size(32), message::binary-size(size)>> = packet
    request_header = request_header(1)

    assert {:ok, _header, _size, %MetadataResponse{} = response} =
             Parser.parse_kafka_response_for_request(request_header, message)

    inlets = InletManager.list_inlets()

    Enum.each(response.brokers, fn broker ->
      node_id = broker.node_id

      ## All hosts should be changed to "localhost"
      assert broker.host == "localhost"

      ## All ports should be changed to inlet ports of base + node_id
      assert broker.port == base_port + node_id

      ## There should be an inlet for each node id
      assert {:ok, _inlet} = Map.fetch(inlets, node_id)

      ## There should be an outlet for each node id
      pid = Ockam.Node.whereis(outlet_prefix <> to_string(node_id))
      assert is_pid(pid)
    end)
  end

  def make_metadata_request(correlation_id) do
    request = %MetadataRequest{
      api_version: @api_version,
      topics: nil,
      allow_auto_topic_creation: false,
      include_topic_authorized_operations: false,
      tagged_fields: %{}
    }

    header = request_header(correlation_id)

    {:ok, request_binary} = RequestFormatter.format(request)
    {:ok, header_binary} = Formatter.format_request_header(header)

    message = header_binary <> request_binary

    size = byte_size(message)

    <<size::signed-big-integer-size(32), message::binary>>
  end

  def request_header(correlation_id) do
    %RequestHeader{
      header_version: 2,
      api_key: @api_metadata,
      api_version: @api_version,
      correlation_id: correlation_id,
      client_id: "test_client",
      tagged_fields: %{}
    }
  end
end
