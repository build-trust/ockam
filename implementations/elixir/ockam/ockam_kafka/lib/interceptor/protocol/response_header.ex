defmodule Ockam.Kafka.Interceptor.Protocol.ResponseHeader do
  @moduledoc """
  Struct representing kafka response header
  """
  alias Ockam.Kafka.Interceptor.Protocol.RequestHeader

  defstruct [:header_version, :api_key, :api_version, :correlation_id, :client_id, :tagged_fields]

  def from_request_header(%RequestHeader{
        api_key: api_key,
        api_version: api_version,
        correlation_id: correlation_id,
        client_id: client_id
      }) do
    %__MODULE__{
      api_key: api_key,
      api_version: api_version,
      correlation_id: correlation_id,
      client_id: client_id
    }
  end
end
