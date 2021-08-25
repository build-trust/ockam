defmodule Ockam.Examples.Stream.IndexApi do
  # credo:disable-for-this-file Credo.Check.Design.AliasUsage
  @moduledoc false

  alias Ockam.Message
  alias Ockam.Protocol.Stream.Index, as: IndexProtocol
  alias Ockam.Workers.Call

  @dialyzer :no_return

  def service_route() do
    tcp_address = Ockam.Transport.TCPAddress.new({127, 0, 0, 1}, 4000)
    [tcp_address, "stream_kafka_index"]
  end

  def outline() do
    Ockam.Examples.Stream.IndexApi.init()

    Ockam.Examples.Stream.IndexApi.get_index("my_api_stream", "i_am_consumer")
    Ockam.Examples.Stream.IndexApi.save_index("my_api_stream", "i_am_consumer", 1)

    Ockam.Examples.Stream.IndexApi.get_index("my_api_stream", "i_am_consumer")

    Ockam.Examples.Stream.IndexApi.get_index("my_api_stream", "i_am_consumer_2")
  end

  def get_index(stream_name, client_id) do
    get_index_request = %{
      onward_route: service_route(),
      payload:
        Ockam.Protocol.encode_payload(
          IndexProtocol,
          :request,
          {:get,
           %{
             stream_name: stream_name,
             client_id: client_id
           }}
        )
    }

    response = Call.call(get_index_request)

    payload = Message.payload(response)

    Ockam.Protocol.decode_payload(IndexProtocol, :response, payload)
  end

  def save_index(stream_name, client_id, index) do
    save_request = %{
      onward_route: service_route(),
      return_route: [],
      payload:
        Ockam.Protocol.encode_payload(
          IndexProtocol,
          :request,
          {:save,
           %{
             stream_name: stream_name,
             client_id: client_id,
             index: index
           }}
        )
    }

    Ockam.Router.route(save_request)
  end

  def init() do
    ensure_tcp()
  end

  def ensure_tcp() do
    Ockam.Transport.TCP.start()
  end
end
