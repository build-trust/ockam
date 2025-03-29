defmodule Ockam.Services.GrpcForwarder do
  @moduledoc false

  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Worker

  require Logger

  defmodule GrpcRequest do
    @moduledoc """
    gRPC request. This request is created by a grpcClient and can be forwarded to a gRPC server.
    This struct is currently being used to receive telemetry data from another node via a secure channel
    and forward it to an OpenTelemetry collector.
    """
    use TypedStruct

    typedstruct do
      plugin(Ockam.TypedCBOR.Plugin)
      field(:method, String.t(), minicbor: [key: 1])
      field(:path, String.t(), minicbor: [key: 2])

      field(:version, :http09 | :http10 | :http11 | :http2 | :http3,
        minicbor: [key: 3, schema: {:enum, [http09: 0, http10: 1, http11: 2, http2: 3, http3: 4]}]
      )

      field(:headers, list(list(String.t())), minicbor: [key: 4])
      field(:body, :binary, minicbor: [key: 5])
    end
  end

  @doc """
  Start the service by opening a channel to the grpc endpoint.
  """
  @impl true
  def setup(options, state) do
    grpc_endpoint = Keyword.get(options, :grpc_endpoint, "http://localhost:4317")
    {:ok, channel} = GRPC.Stub.connect(grpc_endpoint)
    Logger.debug("Starting a grpc forwarder to: #{grpc_endpoint}")

    state =
      state
      |> Map.put(:grpc_endpoint, grpc_endpoint)
      |> Map.put(:channel, channel)

    {:ok, state}
  end

  @doc """
  A message sent to this service is first encoded as an Ockam Request, where the body of the request is the gRPC request.
  In order to forward that request to the grpc endpoint, we need to act as a gRPC client and:
    - Set the appropriate gRPC headers
    - Pass the request path (it should look like `opentelemetry.proto.collector.trace.v1.TraceService/Export`).

  In principle the gRPC request could specify a method other than POST but for now we only support POST.

  We eventually send a reply with an empty body. In a general case, we should provide a better reply but for our
  current telemetry use case this is good enough.
  """
  @impl true
  def handle_message(message, state) do
    {:ok, decoded_request} = Ockam.API.Request.decode(message.payload)
    {:ok, grpc_request, _unused} = GrpcRequest.decode(decoded_request.body)

    channel = Map.get(state, :channel)

    headers =
      GRPC.Transport.HTTP2.client_headers_without_reserved(%{
        channel: channel,
        codec: channel.codec,
        compressor: GRPC.Compressor.Gzip,
        accepted_compressors: nil,
        headers: grpc_request.headers |> Enum.map(fn [a, b] -> {a, b} end)
      })

    :gun.post(channel.adapter_payload.conn_pid, grpc_request.path, headers, grpc_request.body)

    reply = Message.reply(message, state.address, <<>>)
    Worker.route(reply, state)

    {:ok, state}
  end

  @impl true
  def handle_info(_unused, state) do
    {:noreply, state}
  end
end
