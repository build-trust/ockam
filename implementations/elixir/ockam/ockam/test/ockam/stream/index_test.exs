defmodule Ockam.Stream.Index.Tests do
  use ExUnit.Case, async: true

  doctest Ockam.Stream.Index.Service

  alias Ockam.Stream.Index.Service

  alias Ockam.Workers.Call, as: CallHelper

  require Logger

  describe "Index service" do
    setup do
      {:ok, service} = Service.create([])
      [service: service]
    end

    test "get index when index doesn't exist returns undefined", %{service: service} do
      client_id = "test_client"
      stream_name = "my_stream"

      get_req =
        Ockam.Protocol.encode_payload(
          Ockam.Protocol.Stream.Index,
          :request,
          {:get,
           %{
             client_id: client_id,
             stream_name: stream_name
           }}
        )

      %{payload: response} = CallHelper.call(%{onward_route: [service], payload: get_req})

      assert {:ok, %{client_id: ^client_id, stream_name: ^stream_name, index: :undefined}} =
               Ockam.Protocol.decode_payload(Ockam.Protocol.Stream.Index, :response, response)
    end

    test "get index returns saved index", %{service: service} do
      client_id = "test_client"
      stream_name = "my_stream"
      index = 10

      save_req =
        Ockam.Protocol.encode_payload(
          Ockam.Protocol.Stream.Index,
          :request,
          {:save,
           %{
             client_id: client_id,
             stream_name: stream_name,
             index: index
           }}
        )

      Ockam.Router.route(%{
        onward_route: [service],
        payload: save_req,
        return_route: ["/dev/null"]
      })

      get_req =
        Ockam.Protocol.encode_payload(
          Ockam.Protocol.Stream.Index,
          :request,
          {:get,
           %{
             client_id: client_id,
             stream_name: stream_name
           }}
        )

      %{payload: response} = CallHelper.call(%{onward_route: [service], payload: get_req})

      assert {:ok, %{client_id: ^client_id, stream_name: ^stream_name, index: ^index}} =
               Ockam.Protocol.decode_payload(Ockam.Protocol.Stream.Index, :response, response)
    end

    test "get index returns saved index for a partition", %{service: service} do
      client_id = "test_client"
      stream_name = "my_stream"
      index = 10
      partition = 5

      save_req =
        Ockam.Protocol.encode_payload(
          Ockam.Protocol.Stream.Partitioned.Index,
          :request,
          {:save,
           %{
             client_id: client_id,
             stream_name: stream_name,
             partition: partition,
             index: index
           }}
        )

      Ockam.Router.route(%{
        onward_route: [service],
        payload: save_req,
        return_route: ["/dev/null"]
      })

      get_req =
        Ockam.Protocol.encode_payload(
          Ockam.Protocol.Stream.Partitioned.Index,
          :request,
          {:get,
           %{
             client_id: client_id,
             stream_name: stream_name,
             partition: partition
           }}
        )

      %{payload: response} = CallHelper.call(%{onward_route: [service], payload: get_req})

      assert {:ok,
              %{
                client_id: ^client_id,
                stream_name: ^stream_name,
                index: ^index,
                partition: ^partition
              }} =
               Ockam.Protocol.decode_payload(
                 Ockam.Protocol.Stream.Partitioned.Index,
                 :response,
                 response
               )
    end

    test "get index returns higher index", %{service: service} do
      client_id = "test_client"
      stream_name = "my_stream"

      save_req_10 =
        Ockam.Protocol.encode_payload(
          Ockam.Protocol.Stream.Index,
          :request,
          {:save,
           %{
             client_id: client_id,
             stream_name: stream_name,
             index: 10
           }}
        )

      Ockam.Router.route(%{
        onward_route: [service],
        payload: save_req_10,
        return_route: ["/dev/null"]
      })

      save_req_5 =
        Ockam.Protocol.encode_payload(
          Ockam.Protocol.Stream.Index,
          :request,
          {:save,
           %{
             client_id: client_id,
             stream_name: stream_name,
             index: 5
           }}
        )

      Ockam.Router.route(%{
        onward_route: [service],
        payload: save_req_5,
        return_route: ["/dev/null"]
      })

      get_req =
        Ockam.Protocol.encode_payload(
          Ockam.Protocol.Stream.Index,
          :request,
          {:get,
           %{
             client_id: client_id,
             stream_name: stream_name
           }}
        )

      %{payload: response} = CallHelper.call(%{onward_route: [service], payload: get_req})

      assert {:ok, %{client_id: ^client_id, stream_name: ^stream_name, index: 10}} =
               Ockam.Protocol.decode_payload(Ockam.Protocol.Stream.Index, :response, response)
    end
  end
end
