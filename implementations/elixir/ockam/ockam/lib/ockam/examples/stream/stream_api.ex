defmodule Ockam.Examples.Stream.StreamApi do
  # credo:disable-for-this-file Credo.Check.Design.AliasUsage
  @moduledoc false

  alias Ockam.Message
  alias Ockam.Protocol.Stream, as: StreamProtocol
  alias Ockam.Workers.Call

  @dialyzer :no_return

  def service_route() do
    tcp_address = Ockam.Transport.TCPAddress.new({127, 0, 0, 1}, 4000)
    [tcp_address, "stream_kafka"]
  end

  def outline() do
    Ockam.Transport.TCP.start()

    Ockam.Examples.Stream.StreamApi.create_stream("my_api_stream")

    Ockam.Examples.Stream.StreamApi.push_to_stream("my_api_stream", "HI 1!")
    Ockam.Examples.Stream.StreamApi.push_to_stream("my_api_stream", "HI 2!")

    Ockam.Examples.Stream.StreamApi.pull_from_stream("my_api_stream", 0, 1)
    Ockam.Examples.Stream.StreamApi.pull_from_stream("my_api_stream", 0, 10)

    Ockam.Examples.Stream.StreamApi.pull_from_stream("my_api_stream", 1, 1)
  end

  def create_stream(stream_name) do
    create_stream_request = %{
      onward_route: service_route(),
      payload:
        Ockam.Protocol.encode_payload(StreamProtocol.Create, :request, %{stream_name: stream_name})
    }

    response = Call.call(create_stream_request)
    Message.return_route(response)
  end

  def push_to_stream(stream_name, data) do
    stream_route = create_stream(stream_name)

    push_request = %{
      onward_route: stream_route,
      payload:
        Ockam.Protocol.encode_payload(StreamProtocol.Push, :request, %{request_id: 1, data: data})
    }

    push_response = Call.call(push_request)

    payload = Message.payload(push_response)

    Ockam.Protocol.decode_payload(StreamProtocol.Push, :response, payload)
  end

  def pull_from_stream(stream_name, index, limit) do
    stream_route = create_stream(stream_name)

    pull_request = %{
      onward_route: stream_route,
      payload:
        Ockam.Protocol.encode_payload(StreamProtocol.Pull, :request, %{
          request_id: 1,
          index: index,
          limit: limit
        })
    }

    pull_response = Call.call(pull_request)

    payload = Message.payload(pull_response)

    Ockam.Protocol.decode_payload(StreamProtocol.Pull, :response, payload)
  end
end
