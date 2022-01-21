defmodule Ockam.Stream.Workers.Tests do
  use ExUnit.Case, async: true

  doctest Ockam.Stream.Workers.Service
  doctest Ockam.Stream.Workers.Stream

  alias Ockam.Stream.Workers.Service

  alias Ockam.Workers.Call, as: CallHelper

  require Logger

  describe "Stream service" do
    setup do
      {:ok, service} = Service.create([])
      [service: service]
    end

    test "can create a named stream", %{service: service} do
      stream_name = "my_stream"

      %{payload: response} =
        CallHelper.call(%{
          onward_route: [service],
          payload:
            Ockam.Protocol.encode_payload(Ockam.Protocol.Stream.Create, :request, %{
              stream_name: stream_name
            })
        })

      assert {:ok, %{stream_name: ^stream_name}} =
               Ockam.Protocol.decode_payload(Ockam.Protocol.Stream.Create, :response, response)
    end

    test "can create a partitioned stream", %{service: service} do
      stream_name = "my_stream"

      [%{payload: response1}, %{payload: response2}] =
        CallHelper.call_multi(
          %{
            onward_route: [service],
            payload:
              Ockam.Protocol.encode_payload(Ockam.Protocol.Stream.Partitioned.Create, :request, %{
                stream_name: stream_name,
                partitions: 2
              })
          },
          2
        )

      assert {:ok, %{stream_name: ^stream_name, partition: partition1}} =
               Ockam.Protocol.decode_payload(
                 Ockam.Protocol.Stream.Partitioned.Create,
                 :response,
                 response1
               )

      assert {:ok, %{stream_name: ^stream_name, partition: partition2}} =
               Ockam.Protocol.decode_payload(
                 Ockam.Protocol.Stream.Partitioned.Create,
                 :response,
                 response2
               )

      assert [0, 1] == Enum.sort([partition1, partition2])
    end

    test "can create an anonymous stream", %{service: service} do
      %{payload: response} =
        CallHelper.call(%{
          onward_route: [service],
          payload:
            Ockam.Protocol.encode_payload(Ockam.Protocol.Stream.Create, :request, %{
              stream_name: :undefined
            })
        })

      assert {:ok, %{stream_name: stream_name}} =
               Ockam.Protocol.decode_payload(Ockam.Protocol.Stream.Create, :response, response)

      assert "generated" <> _name = stream_name
    end

    test "stream creation is idempotent", %{service: service} do
      stream_name = "my_stream_idempotent"

      %{payload: response, return_route: return_route1} =
        CallHelper.call(%{
          onward_route: [service],
          payload:
            Ockam.Protocol.encode_payload(Ockam.Protocol.Stream.Create, :request, %{
              stream_name: stream_name
            })
        })

      assert {:ok, %{stream_name: ^stream_name}} =
               Ockam.Protocol.decode_payload(Ockam.Protocol.Stream.Create, :response, response)

      streams_1 = Ockam.Node.whereis(service) |> :sys.get_state() |> Map.get(:streams)

      %{payload: response, return_route: return_route2} =
        CallHelper.call(%{
          onward_route: [service],
          payload:
            Ockam.Protocol.encode_payload(Ockam.Protocol.Stream.Create, :request, %{
              stream_name: stream_name
            })
        })

      assert {:ok, %{stream_name: ^stream_name}} =
               Ockam.Protocol.decode_payload(Ockam.Protocol.Stream.Create, :response, response)

      streams_2 = Ockam.Node.whereis(service) |> :sys.get_state() |> Map.get(:streams)

      assert streams_1 == streams_2
      assert return_route1 == return_route2
    end
  end

  describe "Stream instance" do
    setup do
      stream_name = "my_stream"

      {:ok, stream} =
        Ockam.Stream.Workers.Stream.create(
          [
            reply_route: ["/dev/null"],
            stream_name: stream_name,
            partition: 0
          ] ++ Service.stream_options([])
        )

      [stream: stream]
    end

    test "push message", %{stream: stream} do
      request_id = :rand.uniform(100)
      data = "message"

      push_req =
        Ockam.Protocol.encode_payload(Ockam.Protocol.Stream.Push, :request, %{
          request_id: request_id,
          data: data
        })

      %{payload: response} = CallHelper.call(%{onward_route: [stream], payload: push_req})

      assert {:ok, %{request_id: ^request_id, status: :ok, index: _index}} =
               Ockam.Protocol.decode_payload(Ockam.Protocol.Stream.Push, :response, response)
    end

    test "pull message", %{stream: stream} do
      request_id = :rand.uniform(100)
      data = "message"

      push_req =
        Ockam.Protocol.encode_payload(Ockam.Protocol.Stream.Push, :request, %{
          request_id: request_id,
          data: data
        })

      %{payload: push_response} = CallHelper.call(%{onward_route: [stream], payload: push_req})

      {:ok, %{request_id: ^request_id, status: :ok, index: index}} =
        Ockam.Protocol.decode_payload(Ockam.Protocol.Stream.Push, :response, push_response)

      pull_request_id = :rand.uniform(100)

      pull_req =
        Ockam.Protocol.encode_payload(Ockam.Protocol.Stream.Pull, :request, %{
          request_id: pull_request_id,
          index: 0,
          limit: 10
        })

      %{payload: response} = CallHelper.call(%{onward_route: [stream], payload: pull_req})

      assert {:ok, %{request_id: ^pull_request_id, messages: messages}} =
               Ockam.Protocol.decode_payload(Ockam.Protocol.Stream.Pull, :response, response)

      assert [%{index: ^index, data: ^data}] = messages
    end

    test "pull multiple messages", %{stream: stream} do
      request_id = :rand.uniform(100)
      data = "message"

      Enum.map(:lists.seq(0, 100), fn n ->
        push_req =
          Ockam.Protocol.encode_payload(Ockam.Protocol.Stream.Push, :request, %{
            request_id: request_id,
            data: "#{data}_#{n}"
          })

        CallHelper.call(%{onward_route: [stream], payload: push_req})
      end)

      pull_request_id = :rand.uniform(100)

      pull_req =
        Ockam.Protocol.encode_payload(Ockam.Protocol.Stream.Pull, :request, %{
          request_id: pull_request_id,
          index: 0,
          limit: 10
        })

      %{payload: response} = CallHelper.call(%{onward_route: [stream], payload: pull_req})

      assert {:ok, %{request_id: ^pull_request_id, messages: messages}} =
               Ockam.Protocol.decode_payload(Ockam.Protocol.Stream.Pull, :response, response)

      expected_msgs = Enum.map(:lists.seq(0, 9), fn n -> "#{data}_#{n}" end)

      assert expected_msgs ==
               messages
               |> Enum.sort_by(fn %{index: index} -> index end)
               |> Enum.map(fn %{data: data} -> data end)

      pull_request_id = :rand.uniform(100)

      pull_req =
        Ockam.Protocol.encode_payload(Ockam.Protocol.Stream.Pull, :request, %{
          request_id: pull_request_id,
          index: 10,
          limit: 10
        })

      %{payload: response} = CallHelper.call(%{onward_route: [stream], payload: pull_req})

      assert {:ok, %{request_id: ^pull_request_id, messages: messages}} =
               Ockam.Protocol.decode_payload(Ockam.Protocol.Stream.Pull, :response, response)

      expected_msgs = Enum.map(:lists.seq(10, 19), fn n -> "#{data}_#{n}" end)

      assert expected_msgs ==
               messages
               |> Enum.sort_by(fn %{index: index} -> index end)
               |> Enum.map(fn %{data: data} -> data end)
    end
  end
end
