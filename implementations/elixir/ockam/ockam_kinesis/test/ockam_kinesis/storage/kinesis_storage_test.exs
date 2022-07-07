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

  describe "save/4" do
    test "puts message to an AWS Kinesis stream, returns sequence number and stores it in state" do
      stream_name = "stream_name"
      partition = 0
      sequence_number_for_ordering = "49592407930728695436502186699740292765095159714163982338"
      hash_key = "0"

      state = %State{
        hash_key: hash_key,
        sequence_number_for_ordering: sequence_number_for_ordering
      }

      message = "message"
      new_sequence_number = "49596124085897508159438713510240079964989152308217511954"

      expect(ExAwsMock, :request, 1, fn :post, _url, body, headers, _opts ->
        assert {"x-amz-target", "Kinesis_20131202.PutRecord"} in headers

        expected_data = Base.encode64(message)

        assert %{
                 "Data" => ^expected_data,
                 "ExplicitHashKey" => ^hash_key,
                 "SequenceNumberForOrdering" => ^sequence_number_for_ordering,
                 "StreamName" => ^stream_name
               } = Jason.decode!(body)

        {:ok,
         %{
           status_code: 200,
           body: Jason.encode!(%{"SequenceNumber" => new_sequence_number})
         }}
      end)

      assert {{:ok, String.to_integer(new_sequence_number)},
              %{state | sequence_number_for_ordering: new_sequence_number}} ==
               Kinesis.save(stream_name, partition, message, state)
    end

    test "propagates error and state if call to AWS fails" do
      stream_name = "stream_name"
      partition = 0
      state = %State{}
      message = "message"

      error_type = "ResourceNotFoundException"
      error_message = "Stream stream_name under account 000000000000 not found."

      expect(ExAwsMock, :request, 1, fn :post, _url, _body, _headers, _opts ->
        {:ok,
         %{
           status_code: 400,
           body: Jason.encode!(%{"__type" => error_type, "message" => error_message})
         }}
      end)

      assert {{:error, {error_type, error_message}}, state} ==
               Kinesis.save(stream_name, partition, message, state)
    end
  end

  describe "fetch/5" do
    setup do
      state = %State{
        initial_sequence_number:
          49_631_158_273_630_243_944_238_988_867_078_862_282_663_492_862_412_324_866
      }

      %{
        stream_name: "stream_name",
        limit: 1,
        partition: 1,
        message: "message",
        state: state,
        shard_iterator:
          "AAAAAAAAAAG99G9ASBqrYT9HilCAttTeDqzfuoFqqufxbFVoHwc1tKEPkjD6OtvGQR4Lxi5eocHDJeYD+xizBtF3KN1jy+wy7CzU14KwMWoOxKsHInDpqXKyopkeSHL6QkoSrkjIMtHGRkweqcsBlcEfCK5uMzS8h03fRX6UzzYusMGIJFjubgKS4qsd75aD7xA0VCZltyhpzNclBj047VPE3y8RtDLjcPWskASaBKbi1A4DT7mi/g=="
      }
    end

    test "with index lower than or equal to initial sequence number reads from the stream start",
         %{
           stream_name: stream_name,
           limit: limit,
           partition: partition,
           message: message,
           state: state,
           shard_iterator: shard_iterator
         } do
      index = 0
      sequence_number = 49_631_158_273_630_243_944_238_988_869_702_231_311_227_243_256_766_005_250

      expect(ExAwsMock, :request, 1, fn :post, _url, body, headers, _opts ->
        expected_shard_id = "shardId-00000000000#{partition}"
        expected_sequence_number = to_string(state.initial_sequence_number)

        assert %{
                 "StreamName" => ^stream_name,
                 "ShardId" => ^expected_shard_id,
                 "ShardIteratorType" => "AFTER_SEQUENCE_NUMBER",
                 "StartingSequenceNumber" => ^expected_sequence_number
               } = Jason.decode!(body)

        assert {"x-amz-target", "Kinesis_20131202.GetShardIterator"} in headers

        {:ok,
         %{
           status_code: 200,
           body: Jason.encode!(%{"ShardIterator" => shard_iterator})
         }}
      end)

      expect(ExAwsMock, :request, 1, fn :post, _url, body, headers, _opts ->
        assert %{
                 "ShardIterator" => shard_iterator,
                 "Limit" => limit
               } == Jason.decode!(body)

        assert {"x-amz-target", "Kinesis_20131202.GetRecords"} in headers

        response_body = %{
          "MillisBehindLatest" => 0,
          "NextShardIterator" =>
            "AAAAAAAAAAE54atsZujXQa7v1OfgGKakwXhL3M915YyqTj6KbFkEDOQLhdkoDIfA0Nrhl62zLYKCaF8MtqMUDMYFL8h/8rGWhYmjbS+RcZ0EKF6iNM+HWtncwLvp8fEjlXXFh+rlLUuV0bbKhz6fN6jfdD8uZPefZyF/+gmgZuAN+gs1YeaYWZ4S1eZS2WXYw5DEowuY8obSnnrcnNGhMhhmDv4R4Mr0XnxNqkn3D8xsgQ8MKo+7yQ==",
          "Records" => [
            %{
              "ApproximateArrivalTimestamp" => 1_657_178_345.737,
              "Data" => Base.encode64(message),
              "PartitionKey" => "ukurk2jp3qea7r7m66q2xypwgy3xa6oa",
              "SequenceNumber" => "#{sequence_number}"
            }
          ]
        }

        {:ok,
         %{
           status_code: 200,
           body: Jason.encode!(response_body)
         }}
      end)

      new_state = %{state | previous_index: index, previous_sequence_number: sequence_number}

      assert {{:ok, [%{index: sequence_number, data: message}]}, new_state} ==
               Kinesis.fetch(stream_name, partition, index, limit, state)
    end

    test "with index larger than initial sequence number returns message at index",
         %{
           stream_name: stream_name,
           limit: limit,
           partition: partition,
           message: message,
           state: state,
           shard_iterator: shard_iterator
         } do
      index = 49_631_158_273_630_243_944_238_988_869_702_231_311_227_243_256_766_005_250

      expect(ExAwsMock, :request, 1, fn :post, _url, body, headers, _opts ->
        expected_shard_id = "shardId-00000000000#{partition}"
        expected_sequence_number = "#{index}"

        assert %{
                 "StreamName" => ^stream_name,
                 "ShardId" => ^expected_shard_id,
                 "ShardIteratorType" => "AT_SEQUENCE_NUMBER",
                 "StartingSequenceNumber" => ^expected_sequence_number
               } = Jason.decode!(body)

        assert {"x-amz-target", "Kinesis_20131202.GetShardIterator"} in headers

        {:ok,
         %{
           status_code: 200,
           body: Jason.encode!(%{"ShardIterator" => shard_iterator})
         }}
      end)

      expect(ExAwsMock, :request, 1, fn :post, _url, body, headers, _opts ->
        assert %{
                 "ShardIterator" => shard_iterator,
                 "Limit" => limit
               } == Jason.decode!(body)

        assert {"x-amz-target", "Kinesis_20131202.GetRecords"} in headers

        response_body = %{
          "MillisBehindLatest" => 0,
          "NextShardIterator" =>
            "AAAAAAAAAAE54atsZujXQa7v1OfgGKakwXhL3M915YyqTj6KbFkEDOQLhdkoDIfA0Nrhl62zLYKCaF8MtqMUDMYFL8h/8rGWhYmjbS+RcZ0EKF6iNM+HWtncwLvp8fEjlXXFh+rlLUuV0bbKhz6fN6jfdD8uZPefZyF/+gmgZuAN+gs1YeaYWZ4S1eZS2WXYw5DEowuY8obSnnrcnNGhMhhmDv4R4Mr0XnxNqkn3D8xsgQ8MKo+7yQ==",
          "Records" => [
            %{
              "ApproximateArrivalTimestamp" => 1_657_178_345.737,
              "Data" => Base.encode64(message),
              "PartitionKey" => "ukurk2jp3qea7r7m66q2xypwgy3xa6oa",
              "SequenceNumber" => "#{index}"
            }
          ]
        }

        {:ok,
         %{
           status_code: 200,
           body: Jason.encode!(response_body)
         }}
      end)

      new_state = %{state | previous_index: index, previous_sequence_number: index}

      assert {{:ok, [%{index: index, data: message}]}, new_state} ==
               Kinesis.fetch(stream_name, partition, index, limit, state)
    end

    test "with index larger than previous sequence number returns next message",
         %{
           stream_name: stream_name,
           limit: limit,
           partition: partition,
           message: message,
           state: state,
           shard_iterator: shard_iterator
         } do
      index = 49_631_158_273_630_243_944_238_988_869_702_231_311_227_243_256_766_005_251
      previous_sequence_number = index - 1

      new_sequence_number =
        49_631_158_273_630_243_944_238_988_869_966_986_065_722_869_104_978_690_050

      state = %{state | previous_sequence_number: previous_sequence_number}

      expect(ExAwsMock, :request, 1, fn :post, _url, body, headers, _opts ->
        expected_shard_id = "shardId-00000000000#{partition}"
        expected_sequence_number = "#{previous_sequence_number}"

        assert %{
                 "StreamName" => ^stream_name,
                 "ShardId" => ^expected_shard_id,
                 "ShardIteratorType" => "AFTER_SEQUENCE_NUMBER",
                 "StartingSequenceNumber" => ^expected_sequence_number
               } = Jason.decode!(body)

        assert {"x-amz-target", "Kinesis_20131202.GetShardIterator"} in headers

        {:ok,
         %{
           status_code: 200,
           body: Jason.encode!(%{"ShardIterator" => shard_iterator})
         }}
      end)

      expect(ExAwsMock, :request, 1, fn :post, _url, body, headers, _opts ->
        assert %{
                 "ShardIterator" => shard_iterator,
                 "Limit" => limit
               } == Jason.decode!(body)

        assert {"x-amz-target", "Kinesis_20131202.GetRecords"} in headers

        response_body = %{
          "MillisBehindLatest" => 0,
          "NextShardIterator" =>
            "AAAAAAAAAAE54atsZujXQa7v1OfgGKakwXhL3M915YyqTj6KbFkEDOQLhdkoDIfA0Nrhl62zLYKCaF8MtqMUDMYFL8h/8rGWhYmjbS+RcZ0EKF6iNM+HWtncwLvp8fEjlXXFh+rlLUuV0bbKhz6fN6jfdD8uZPefZyF/+gmgZuAN+gs1YeaYWZ4S1eZS2WXYw5DEowuY8obSnnrcnNGhMhhmDv4R4Mr0XnxNqkn3D8xsgQ8MKo+7yQ==",
          "Records" => [
            %{
              "ApproximateArrivalTimestamp" => 1_657_178_345.737,
              "Data" => Base.encode64(message),
              "PartitionKey" => "ukurk2jp3qea7r7m66q2xypwgy3xa6oa",
              "SequenceNumber" => "#{new_sequence_number}"
            }
          ]
        }

        {:ok,
         %{
           status_code: 200,
           body: Jason.encode!(response_body)
         }}
      end)

      new_state = %{
        state
        | previous_index: index,
          previous_sequence_number: new_sequence_number
      }

      assert {{:ok, [%{index: new_sequence_number, data: message}]}, new_state} ==
               Kinesis.fetch(stream_name, partition, index, limit, state)
    end

    test "at the end of the stream fetches records using persisted shard iterator",
         %{
           stream_name: stream_name,
           limit: limit,
           partition: partition,
           state: state,
           shard_iterator: shard_iterator
         } do
      index = 49_631_158_273_630_243_944_238_988_869_702_231_311_227_243_256_766_005_251
      state = %{state | previous_index: index, next_shard_iterator: shard_iterator}

      next_shard_iterator =
        "AAAAAAAAAAE54atsZujXQa7v1OfgGKakwXhL3M915YyqTj6KbFkEDOQLhdkoDIfA0Nrhl62zLYKCaF8MtqMUDMYFL8h/8rGWhYmjbS+RcZ0EKF6iNM+HWtncwLvp8fEjlXXFh+rlLUuV0bbKhz6fN6jfdD8uZPefZyF/+gmgZuAN+gs1YeaYWZ4S1eZS2WXYw5DEowuY8obSnnrcnNGhMhhmDv4R4Mr0XnxNqkn3D8xsgQ8MKo+7yQ=="

      expect(ExAwsMock, :request, 1, fn :post, _url, body, headers, _opts ->
        assert %{
                 "ShardIterator" => shard_iterator,
                 "Limit" => limit
               } == Jason.decode!(body)

        assert {"x-amz-target", "Kinesis_20131202.GetRecords"} in headers

        response_body = %{
          "MillisBehindLatest" => 0,
          "NextShardIterator" => next_shard_iterator,
          "Records" => []
        }

        {:ok,
         %{
           status_code: 200,
           body: Jason.encode!(response_body)
         }}
      end)

      new_state = %{
        state
        | previous_index: index,
          next_shard_iterator: next_shard_iterator
      }

      assert {{:ok, []}, new_state} == Kinesis.fetch(stream_name, partition, index, limit, state)
    end

    test "propagates error and state if call to AWS fails", %{
      stream_name: stream_name,
      limit: limit,
      partition: partition,
      state: state
    } do
      error_type = "ResourceNotFoundException"
      error_message = "Stream #{stream_name} under account 000000000000 not found."

      expect(ExAwsMock, :request, 1, fn :post, _url, _body, _headers, _opts ->
        {:ok,
         %{
           status_code: 400,
           body: Jason.encode!(%{"__type" => error_type, "message" => error_message})
         }}
      end)

      assert {{:error, {error_type, error_message}}, state} ==
               Kinesis.fetch(stream_name, partition, 0, limit, state)
    end
  end
end
