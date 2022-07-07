defmodule Ockam.Stream.Storage.KinesisTest do
  use ExUnit.Case, async: true

  import Mox

  alias Ockam.Stream.Storage.Kinesis
  alias Ockam.Stream.Storage.Kinesis.State

  setup :verify_on_exit!

  describe "init_stream/3" do
    test "creates stream in Kinesis, awaits activation, returns state" do
      stream_name = "stream_name"
      partitions = 1
      options = []

      expect(ExAwsMock, :request, 1, fn :post, _url, body, headers, _opts ->
        assert %{"ShardCount" => ^partitions, "StreamName" => ^stream_name} = Jason.decode!(body)
        assert {"x-amz-target", "Kinesis_20131202.CreateStream"} in headers

        {:ok, %{status_code: 200, body: "{}"}}
      end)

      expect(ExAwsMock, :request, 1, fn :post, _url, body, headers, _opts ->
        assert %{"StreamName" => ^stream_name} = Jason.decode!(body)
        assert {"x-amz-target", "Kinesis_20131202.DescribeStreamSummary"} in headers

        {:ok,
         %{
           status_code: 200,
           body: Jason.encode!(%{"StreamDescriptionSummary" => %{"StreamStatus" => "ACTIVE"}})
         }}
      end)

      assert {:ok, %State{options: options}} ==
               Kinesis.init_stream(stream_name, partitions, options)
    end

    test "returns an error if a call to create stream in AWS fails" do
      stream_name = "stream_name"
      partitions = 0
      options = []
      error_type = "ValidationException"

      error_message =
        "1 validation error detected: Value '0' at 'shardCount' failed to satisfy constraint: Member must have value greater than or equal to 1"

      expect(ExAwsMock, :request, 1, fn :post, _url, _body, _headers, _opts ->
        {:ok,
         %{
           status_code: 400,
           body:
             Jason.encode!(%{
               "__type" => error_type,
               "message" => error_message
             })
         }}
      end)

      assert {:error, {error_type, error_message}} ==
               Kinesis.init_stream(stream_name, partitions, options)
    end

    test "retries a call to describe stream if stream is not active" do
      stream_name = "stream_name"
      partitions = 1
      options = []

      expect(ExAwsMock, :request, 1, fn :post, _url, _body, _headers, _opts ->
        {:ok, %{status_code: 200, body: "{}"}}
      end)

      expect(ExAwsMock, :request, 1, fn :post, _url, _body, headers, _opts ->
        assert {"x-amz-target", "Kinesis_20131202.DescribeStreamSummary"} in headers

        {:ok,
         %{
           status_code: 200,
           body: Jason.encode!(%{"StreamDescriptionSummary" => %{"StreamStatus" => "CREATING"}})
         }}
      end)

      expect(ExAwsMock, :request, 1, fn :post, _url, _body, headers, _opts ->
        assert {"x-amz-target", "Kinesis_20131202.DescribeStreamSummary"} in headers

        {:ok,
         %{
           status_code: 200,
           body: Jason.encode!(%{"StreamDescriptionSummary" => %{"StreamStatus" => "ACTIVE"}})
         }}
      end)

      assert {:ok, %State{options: options}} ==
               Kinesis.init_stream(stream_name, partitions, options)
    end

    test "retries an error if call to describe stream fails" do
      stream_name = "stream_name"
      partitions = 1
      options = []
      error_type = "SomeError"
      error_message = "some error description"

      expect(ExAwsMock, :request, 1, fn :post, _url, _body, _headers, _opts ->
        {:ok, %{status_code: 200, body: "{}"}}
      end)

      expect(ExAwsMock, :request, 1, fn :post, _url, _body, _headers, _opts ->
        {:ok,
         %{
           status_code: 400,
           body: Jason.encode!(%{"__type" => error_type, "message" => error_message})
         }}
      end)

      assert {:error, {error_type, error_message}} ==
               Kinesis.init_stream(stream_name, partitions, options)
    end
  end

  describe "init_partition/4" do
    test "retrieves shard hash and stores it with empty pointer in the state" do
      stream_name = "stream_name"
      partition = 1
      state = %State{options: [foo: "bar"]}
      options = [bar: "foo"]
      initial_sequence_number = "49631157130404842086765124139391800753617668442628816898"
      hash_key = "hash_key"

      expect(ExAwsMock, :request, 1, fn :post, _url, body, headers, _opts ->
        assert %{"StreamName" => ^stream_name} = Jason.decode!(body)
        assert {"x-amz-target", "Kinesis_20131202.DescribeStream"} in headers

        response = %{
          "StreamDescription" => %{
            "Shards" => [
              %{
                "HashKeyRange" => %{"StartingHashKey" => hash_key},
                "SequenceNumberRange" => %{"StartingSequenceNumber" => initial_sequence_number},
                "ShardId" => "shardId-000000000001"
              }
            ]
          }
        }

        {:ok, %{status_code: 200, body: Jason.encode!(response)}}
      end)

      assert {:ok,
              %State{
                hash_key: hash_key,
                initial_sequence_number: String.to_integer(initial_sequence_number),
                options: [foo: "bar", bar: "foo"]
              }} ==
               Kinesis.init_partition(stream_name, partition, state, options)
    end

    test "propagates error if call to AWS fails" do
      stream_name = "stream_name"
      partition = 1
      state = %State{options: []}
      options = []

      error_type = "ResourceNotFoundException"
      error_message = "Stream stream_name under account 000000000000 not found."

      expect(ExAwsMock, :request, 1, fn :post, _url, _body, _headers, _opts ->
        {:ok,
         %{
           status_code: 400,
           body: Jason.encode!(%{"__type" => error_type, "message" => error_message})
         }}
      end)

      assert {:error, {error_type, error_message}} ==
               Kinesis.init_partition(stream_name, partition, state, options)
    end
  end
end
