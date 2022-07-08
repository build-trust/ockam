defmodule Ockam.KinesisTest do
  use ExUnit.Case, async: true

  import Mox

  alias Ockam.Kinesis

  setup :verify_on_exit!

  describe "create_stream/2" do
    test "returns :ok if call to create stream in AWS is successful" do
      stream_name = "stream_name"
      partitions = 5

      expect(AWSMock, :request, 1, fn :post, _url, body, headers, _opts ->
        assert %{"ShardCount" => ^partitions, "StreamName" => ^stream_name} = Jason.decode!(body)
        assert {"X-Amz-Target", "Kinesis_20131202.CreateStream"} in headers

        {:ok, %{status_code: 200, body: "{}"}}
      end)

      assert :ok == Kinesis.create_stream(stream_name, partitions)
    end

    test "returns :ok if stream already exists" do
      stream_name = "stream_name"
      partitions = 5

      expect(AWSMock, :request, 1, fn :post, _url, _body, _headers, _opts ->
        response_body = %{
          "__type" => "ResourceInUseException",
          "message" => "Stream #{stream_name} under account 00000000000000 already exists."
        }

        {:ok, %{status_code: 400, body: Jason.encode!(response_body)}}
      end)

      assert :ok == Kinesis.create_stream(stream_name, partitions)
    end

    test "propagates error if call to AWS fails for other reason" do
      stream_name = "stream_name"
      partitions = 500

      error = %{
        "__type" => "LimitExceededException",
        "message" =>
          "This request would exceed the shard limit for the account 000000000000 in us-east-1."
      }

      expect(AWSMock, :request, 1, fn :post, _url, _body, _headers, _opts ->
        {:ok, %{status_code: 400, body: Jason.encode!(error)}}
      end)

      assert {:error, error} ==
               Kinesis.create_stream(stream_name, partitions)
    end
  end

  describe "describe_stream/2" do
    test "propagates response on a successful call to AWS" do
      stream_name = "stream_name"

      response = %{
        "StreamDescription" => %{
          "EncryptionType" => "NONE",
          "EnhancedMonitoring" => [%{"ShardLevelMetrics" => []}],
          "HasMoreShards" => false,
          "RetentionPeriodHours" => 24,
          "Shards" => [
            %{
              "HashKeyRange" => %{
                "EndingHashKey" => "340282366920938463463374607431768211455",
                "StartingHashKey" => "0"
              },
              "SequenceNumberRange" => %{
                "StartingSequenceNumber" =>
                  "49631158273630243944238988867078862282663492862412324866"
              },
              "ShardId" => "shardId-000000000000"
            }
          ],
          "StreamARN" => "arn:aws:kinesis:us-east-1:857201052021:stream/test",
          "StreamCreationTimestamp" => 1_657_178_274.0,
          "StreamModeDetails" => %{"StreamMode" => "PROVISIONED"},
          "StreamName" => "#{stream_name}",
          "StreamStatus" => "ACTIVE"
        }
      }

      expect(AWSMock, :request, 1, fn :post, _url, body, headers, _opts ->
        assert %{"StreamName" => ^stream_name} = Jason.decode!(body)
        assert {"X-Amz-Target", "Kinesis_20131202.DescribeStream"} in headers

        {:ok, %{status_code: 200, body: Jason.encode!(response)}}
      end)

      assert {:ok, response} == Kinesis.describe_stream(stream_name, [])
    end

    test "propagates error if call to AWS fails" do
      stream_name = "stream_name"

      error = %{
        "__type" => "ResourceNotFoundException",
        "message" => "Stream #{stream_name} under account 000000000000 not found."
      }

      expect(AWSMock, :request, 1, fn :post, _url, _body, _headers, _opts ->
        {:ok, %{status_code: 400, body: Jason.encode!(error)}}
      end)

      assert {:error, error} == Kinesis.describe_stream(stream_name, [])
    end
  end

  describe "describe_stream_summary/1" do
    test "propagates response on a successful call to AWS" do
      stream_name = "stream_name"

      response = %{
        "StreamDescriptionSummary" => %{
          "ConsumerCount" => 0,
          "EncryptionType" => "NONE",
          "EnhancedMonitoring" => [%{"ShardLevelMetrics" => []}],
          "OpenShardCount" => 1,
          "RetentionPeriodHours" => 24,
          "StreamARN" => "arn:aws:kinesis:us-east-1:857201052021:stream/test",
          "StreamCreationTimestamp" => 1_657_178_274.0,
          "StreamModeDetails" => %{"StreamMode" => "PROVISIONED"},
          "StreamName" => "#{stream_name}",
          "StreamStatus" => "ACTIVE"
        }
      }

      expect(AWSMock, :request, 1, fn :post, _url, body, headers, _opts ->
        assert %{"StreamName" => ^stream_name} = Jason.decode!(body)
        assert {"X-Amz-Target", "Kinesis_20131202.DescribeStreamSummary"} in headers

        {:ok, %{status_code: 200, body: Jason.encode!(response)}}
      end)

      assert {:ok, response} == Kinesis.describe_stream_summary(stream_name)
    end

    test "propagates error if call to AWS fails" do
      stream_name = "stream_name"

      error = %{
        "__type" => "ResourceNotFoundException",
        "message" => "Stream #{stream_name} under account 000000000000 not found."
      }

      expect(AWSMock, :request, 1, fn :post, _url, _body, _headers, _opts ->
        {:ok,
         %{
           status_code: 400,
           body: Jason.encode!(error)
         }}
      end)

      assert {:error, error} == Kinesis.describe_stream_summary(stream_name)
    end
  end

  describe "get_shard_iterator/4" do
    test "converts partition to shard id, calls AWS and returns shard iterator on success" do
      stream_name = "stream_name"
      partition = 10

      shard_iterator =
        "AAAAAAAAAAG99G9ASBqrYT9HilCAttTeDqzfuoFqqufxbFVoHwc1tKEPkjD6OtvGQR4Lxi5eocHDJeYD+xizBtF3KN1jy+wy7CzU14KwMWoOxKsHInDpqXKyopkeSHL6QkoSrkjIMtHGRkweqcsBlcEfCK5uMzS8h03fRX6UzzYusMGIJFjubgKS4qsd75aD7xA0VCZltyhpzNclBj047VPE3y8RtDLjcPWskASaBKbi1A4DT7mi/g=="

      expect(AWSMock, :request, 1, fn :post, _url, body, headers, _opts ->
        expected_shard_id = "shardId-000000000010"

        assert %{
                 "StreamName" => ^stream_name,
                 "ShardId" => ^expected_shard_id
               } = Jason.decode!(body)

        assert {"X-Amz-Target", "Kinesis_20131202.GetShardIterator"} in headers

        {:ok,
         %{
           status_code: 200,
           body: Jason.encode!(%{"ShardIterator" => shard_iterator})
         }}
      end)

      assert {:ok, shard_iterator} ==
               Kinesis.get_shard_iterator(stream_name, partition, :trim_horizon, [])
    end

    test "propagates error if call to AWS fails" do
      stream_name = "stream_name"
      partition = 10

      error = %{
        "__type" => "ResourceNotFoundException",
        "message" => "Stream #{stream_name} under account 000000000000 not found."
      }

      expect(AWSMock, :request, 1, fn :post, _url, _body, _headers, _opts ->
        {:ok, %{status_code: 400, body: Jason.encode!(error)}}
      end)

      assert {:error, error} ==
               Kinesis.get_shard_iterator(stream_name, partition, :trim_horizon, [])
    end
  end

  describe "get_records/2" do
    test "returns messages, `nil` as next shard iterator and latest index if there are messages in the shard" do
      shard_iterator =
        "AAAAAAAAAAG99G9ASBqrYT9HilCAttTeDqzfuoFqqufxbFVoHwc1tKEPkjD6OtvGQR4Lxi5eocHDJeYD+xizBtF3KN1jy+wy7CzU14KwMWoOxKsHInDpqXKyopkeSHL6QkoSrkjIMtHGRkweqcsBlcEfCK5uMzS8h03fRX6UzzYusMGIJFjubgKS4qsd75aD7xA0VCZltyhpzNclBj047VPE3y8RtDLjcPWskASaBKbi1A4DT7mi/g=="

      limit = 2

      message_1 = "message_1"
      message_2 = "message_2"
      message_1_index = 49_631_158_273_630_243_944_238_988_869_702_231_311_227_243_256_766_005_250
      message_2_index = 49_631_158_273_630_243_944_238_988_869_846_093_483_761_396_772_939_759_618

      response = %{
        "MillisBehindLatest" => 13_254_000,
        "NextShardIterator" =>
          "AAAAAAAAAAGLKb1AZgPM91UNS9BIFizZ084fbehO2713xFP3vuwgDA7Lyu8E3w71zYw8ZFEH4CPZtPD+5P2oDXUZqJsTK/ZqjZmscT3xX9z2/Zo1KK3/F4oWWW27+XEnNm9kpLLJoOk3aEiP8l3UVLAnX8UPVeDpd57lL66mffNHoNzwFnZYmTq9gOO33GxY4/Xe9AVOJz/kVLOfw/XLfKk/sohO+Tz+T4C3iq91QTLQ7VAOv3o+CQ==",
        "Records" => [
          %{
            "ApproximateArrivalTimestamp" => 1_657_178_345.737,
            "Data" => Base.encode64(message_1),
            "PartitionKey" => "ukurk2jp3qea7r7m66q2xypwgy3xa6oa",
            "SequenceNumber" => "#{message_1_index}"
          },
          %{
            "ApproximateArrivalTimestamp" => 1_657_178_530.827,
            "Data" => Base.encode64(message_2),
            "PartitionKey" => "4e7pn7jm663gvqpcwaza4xfdqicamwqx",
            "SequenceNumber" => "#{message_2_index}"
          }
        ]
      }

      expect(AWSMock, :request, 1, fn :post, _url, body, headers, _opts ->
        assert %{
                 "ShardIterator" => shard_iterator,
                 "Limit" => limit
               } == Jason.decode!(body)

        assert {"X-Amz-Target", "Kinesis_20131202.GetRecords"} in headers

        {:ok, %{status_code: 200, body: Jason.encode!(response)}}
      end)

      records = [
        %{index: message_1_index, data: message_1},
        %{index: message_2_index, data: message_2}
      ]

      assert {:ok, {records, nil, message_2_index}} ==
               Ockam.Kinesis.get_records(shard_iterator, limit)
    end

    test "if there are no messages in the shard portion iterates until messages are found" do
      shard_iterator =
        "AAAAAAAAAAG99G9ASBqrYT9HilCAttTeDqzfuoFqqufxbFVoHwc1tKEPkjD6OtvGQR4Lxi5eocHDJeYD+xizBtF3KN1jy+wy7CzU14KwMWoOxKsHInDpqXKyopkeSHL6QkoSrkjIMtHGRkweqcsBlcEfCK5uMzS8h03fRX6UzzYusMGIJFjubgKS4qsd75aD7xA0VCZltyhpzNclBj047VPE3y8RtDLjcPWskASaBKbi1A4DT7mi/g=="

      limit = 2

      message_1 = "message_1"
      message_2 = "message_2"
      message_1_index = 49_631_158_273_630_243_944_238_988_869_702_231_311_227_243_256_766_005_250
      message_2_index = 49_631_158_273_630_243_944_238_988_869_846_093_483_761_396_772_939_759_618

      next_shard_iterator =
        "AAAAAAAAAAGLKb1AZgPM91UNS9BIFizZ084fbehO2713xFP3vuwgDA7Lyu8E3w71zYw8ZFEH4CPZtPD+5P2oDXUZqJsTK/ZqjZmscT3xX9z2/Zo1KK3/F4oWWW27+XEnNm9kpLLJoOk3aEiP8l3UVLAnX8UPVeDpd57lL66mffNHoNzwFnZYmTq9gOO33GxY4/Xe9AVOJz/kVLOfw/XLfKk/sohO+Tz+T4C3iq91QTLQ7VAOv3o+CQ=="

      first_response = %{
        "MillisBehindLatest" => 13_254_000,
        "NextShardIterator" => next_shard_iterator,
        "Records" => []
      }

      expect(AWSMock, :request, 1, fn :post, _url, body, headers, _opts ->
        assert %{
                 "ShardIterator" => shard_iterator,
                 "Limit" => limit
               } == Jason.decode!(body)

        assert {"X-Amz-Target", "Kinesis_20131202.GetRecords"} in headers

        {:ok, %{status_code: 200, body: Jason.encode!(first_response)}}
      end)

      second_response = %{
        "MillisBehindLatest" => 0,
        "NextShardIterator" =>
          "AAAAAAAAAAFHpKmd1zLgVlEqkHrkBSYN4FiQcFbWUY7YkGCas7eHLimST3B8x0fKK6V0aPqcT26J4jGWP9rGrjDx76vW2/zVJ1PO8993Gsv1ZgPdrsqmixAb95cyTauMmiYlb8NUD73cxaAW2fd/8gIkoAnkeidp4b7EdizYXSOjmF+5u0pI0D1WI0qUKG+maTgTxtMrXKm+NU95gLFM1BFttT5ujZukM5oy9xTlikVjFRcO8hb5MA==",
        "Records" => [
          %{
            "ApproximateArrivalTimestamp" => 1_657_178_345.737,
            "Data" => Base.encode64(message_1),
            "PartitionKey" => "ukurk2jp3qea7r7m66q2xypwgy3xa6oa",
            "SequenceNumber" => "#{message_1_index}"
          },
          %{
            "ApproximateArrivalTimestamp" => 1_657_178_530.827,
            "Data" => Base.encode64(message_2),
            "PartitionKey" => "4e7pn7jm663gvqpcwaza4xfdqicamwqx",
            "SequenceNumber" => "#{message_2_index}"
          }
        ]
      }

      expect(AWSMock, :request, 1, fn :post, _url, body, headers, _opts ->
        assert %{
                 "ShardIterator" => next_shard_iterator,
                 "Limit" => limit
               } == Jason.decode!(body)

        assert {"X-Amz-Target", "Kinesis_20131202.GetRecords"} in headers

        {:ok, %{status_code: 200, body: Jason.encode!(second_response)}}
      end)

      records = [
        %{index: message_1_index, data: message_1},
        %{index: message_2_index, data: message_2}
      ]

      assert {:ok, {records, nil, message_2_index}} ==
               Ockam.Kinesis.get_records(shard_iterator, limit)
    end

    test "if there are no messages at the end of the stream returns empty list and next shard iterator" do
      shard_iterator =
        "AAAAAAAAAAG99G9ASBqrYT9HilCAttTeDqzfuoFqqufxbFVoHwc1tKEPkjD6OtvGQR4Lxi5eocHDJeYD+xizBtF3KN1jy+wy7CzU14KwMWoOxKsHInDpqXKyopkeSHL6QkoSrkjIMtHGRkweqcsBlcEfCK5uMzS8h03fRX6UzzYusMGIJFjubgKS4qsd75aD7xA0VCZltyhpzNclBj047VPE3y8RtDLjcPWskASaBKbi1A4DT7mi/g=="

      limit = 2

      next_shard_iterator =
        "AAAAAAAAAAGLKb1AZgPM91UNS9BIFizZ084fbehO2713xFP3vuwgDA7Lyu8E3w71zYw8ZFEH4CPZtPD+5P2oDXUZqJsTK/ZqjZmscT3xX9z2/Zo1KK3/F4oWWW27+XEnNm9kpLLJoOk3aEiP8l3UVLAnX8UPVeDpd57lL66mffNHoNzwFnZYmTq9gOO33GxY4/Xe9AVOJz/kVLOfw/XLfKk/sohO+Tz+T4C3iq91QTLQ7VAOv3o+CQ=="

      response = %{
        "MillisBehindLatest" => 0,
        "NextShardIterator" => next_shard_iterator,
        "Records" => []
      }

      expect(AWSMock, :request, 1, fn :post, _url, body, headers, _opts ->
        assert %{
                 "ShardIterator" => shard_iterator,
                 "Limit" => limit
               } == Jason.decode!(body)

        assert {"X-Amz-Target", "Kinesis_20131202.GetRecords"} in headers

        {:ok, %{status_code: 200, body: Jason.encode!(response)}}
      end)

      assert {:ok, {[], next_shard_iterator, nil}} ==
               Ockam.Kinesis.get_records(shard_iterator, limit)
    end

    test "propagates error if call to AWS fails" do
      shard_iterator =
        "AAAAAAAAAAG99G9ASBqrYT9HilCAttTeDqzfuoFqqufxbFVoHwc1tKEPkjD6OtvGQR4Lxi5eocHDJeYD+xizBtF3KN1jy+wy7CzU14KwMWoOxKsHInDpqXKyopkeSHL6QkoSrkjIMtHGRkweqcsBlcEfCK5uMzS8h03fRX6UzzYusMGIJFjubgKS4qsd75aD7xA0VCZltyhpzNclBj047VPE3y8RtDLjcPWskASaBKbi1A4DT7mi/g=="

      limit = 2

      error = %{
        "__type" => "ExpiredIteratorException",
        "message" => "Iterator expired."
      }

      expect(AWSMock, :request, 1, fn :post, _url, _body, _headers, _opts ->
        {:ok,
         %{
           status_code: 400,
           body: Jason.encode!(error)
         }}
      end)

      assert {:error, error} == Ockam.Kinesis.get_records(shard_iterator, limit)
    end
  end

  describe "put_record/4" do
    test "returns message sequence number on successful call to AWS" do
      stream_name = "stream_name"
      message = "message"
      partition_key = "partition_key"
      sequence_number = "49631158273630243944238988869702231311227243256766005250"

      expect(AWSMock, :request, 1, fn :post, _url, body, headers, _opts ->
        assert {"X-Amz-Target", "Kinesis_20131202.PutRecord"} in headers

        expected_data = Base.encode64(message)

        assert %{
                 "Data" => expected_data,
                 "PartitionKey" => partition_key,
                 "StreamName" => stream_name
               } == Jason.decode!(body)

        response = %{
          "SequenceNumber" => sequence_number,
          "EncryptionType" => "NONE",
          "ShardId" => "shardId-000000000000"
        }

        {:ok, %{status_code: 200, body: Jason.encode!(response)}}
      end)

      assert {:ok, sequence_number} ==
               Ockam.Kinesis.put_record(stream_name, message, partition_key, [])
    end

    test "generates random partiton key if not provided" do
      stream_name = "stream_name"
      message = "message"
      partition_key = nil

      expect(AWSMock, :request, 1, fn :post, _url, body, _headers, _opts ->
        assert %{"PartitionKey" => partition_key} = Jason.decode!(body)

        assert is_binary(partition_key)

        response = %{
          "SequenceNumber" => "49631158273630243944238988869702231311227243256766005250",
          "EncryptionType" => "NONE",
          "ShardId" => "shardId-000000000000"
        }

        {:ok, %{status_code: 200, body: Jason.encode!(response)}}
      end)

      assert {:ok, _index} = Ockam.Kinesis.put_record(stream_name, message, partition_key, [])
    end

    test "propagates error if call to AWS fails" do
      stream_name = "stream_name"
      message = "message"
      partition_key = "partition_key"

      error = %{
        "__type" => "ResourceNotFoundException",
        "message" => "Stream #{stream_name} under account 000000000000 not found."
      }

      expect(AWSMock, :request, 1, fn :post, _url, _body, _headers, _opts ->
        {:ok, %{status_code: 400, body: Jason.encode!(error)}}
      end)

      assert {:error, error} == Ockam.Kinesis.put_record(stream_name, message, partition_key, [])
    end
  end

  describe "shard_id/1" do
    test "returns nil if partition number < 0" do
      assert nil == Kinesis.shard_id(-1)
    end

    test "returns AWS ShardId if partition number >= 0" do
      assert "shardId-000000000000" == Kinesis.shard_id(0)
      assert "shardId-000000000001" == Kinesis.shard_id(1)
      assert "shardId-000000000010" == Kinesis.shard_id(10)
      assert "shardId-000000001000" == Kinesis.shard_id(1000)
    end
  end
end
