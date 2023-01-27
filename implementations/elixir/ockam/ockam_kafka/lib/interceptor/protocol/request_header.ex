defmodule Ockam.Kafka.Interceptor.Protocol.RequestHeader do
  @moduledoc """
  Struct representing kafka request header
  """
  defstruct [:header_version, :api_key, :api_version, :correlation_id, :client_id, :tagged_fields]
end
