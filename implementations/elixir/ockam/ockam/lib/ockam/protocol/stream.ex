defmodule Ockam.Protocol.Stream.Index do
  @moduledoc false
  @behaviour Ockam.Protocol

  @impl true
  def protocol() do
    %Ockam.Protocol{
      name: "stream_index",
      ## type Request (IndexGet | IndexSave)
      request: [get: index_get_schema(), save: index_save_schema()],
      response: index_schema()
    }
  end

  # type Index {
  #   client_id: string
  #   stream_name: string
  #   index: optional<uint>
  # }
  def index_schema() do
    {:struct, [client_id: :string, stream_name: :string, index: {:optional, :uint}]}
  end

  # type IndexSave {
  #   client_id: string
  #   stream_name: string
  #   index: uint
  # }
  def index_save_schema() do
    {:struct, [client_id: :string, stream_name: :string, index: :uint]}
  end

  # type IndexGet {
  #   client_id: string
  #   stream_name: string
  # }
  def index_get_schema() do
    {:struct, [client_id: :string, stream_name: :string]}
  end
end

defmodule Ockam.Protocol.Stream.Create do
  @moduledoc false
  @behaviour Ockam.Protocol

  @impl true
  def protocol() do
    %Ockam.Protocol{
      name: "stream_create",
      request: create_schema(),
      ## type Response (Init | Error)
      response: init_schema()
    }
  end

  # type Init {
  #   stream_name: string
  # }
  def init_schema() do
    {:struct, [stream_name: :string]}
  end

  # type CreateStreamRequest {
  #   stream_name: string
  # }
  def create_schema() do
    {:struct, [stream_name: {:optional, :string}]}
  end
end

defmodule Ockam.Protocol.Error do
  @moduledoc false
  @behaviour Ockam.Protocol

  @impl true
  def protocol() do
    %Ockam.Protocol{
      name: "error",
      response: error_schema()
    }
  end

  # type Error {
  #   reason: string
  # }
  def error_schema() do
    {:struct, [reason: :string]}
  end
end

defmodule Ockam.Protocol.Stream.Push do
  @moduledoc false
  @behaviour Ockam.Protocol

  @impl true
  def protocol() do
    %Ockam.Protocol{
      name: "stream_push",
      request: push_request_schema(),
      response: push_confirm_schema()
    }
  end

  # type PushRequest {
  #   request_id: uint
  #   data: data
  # }
  def push_request_schema() do
    {:struct, [request_id: :uint, data: :data]}
  end

  # enum Status {
  #   OK
  #   ERROR
  # }
  # type PushConfirm {
  #   request_id: uint
  #   status: Status,
  #   index: uint
  # }
  def push_confirm_schema() do
    {:struct, [request_id: :uint, status: {:enum, [:ok, :error]}, index: :uint]}
  end
end

defmodule Ockam.Protocol.Stream.Pull do
  @moduledoc false
  @behaviour Ockam.Protocol

  @impl true
  def protocol() do
    %Ockam.Protocol{
      name: "stream_pull",
      request: pull_request_schema(),
      response: pull_response_schema()
    }
  end

  # type PullRequest {
  #   request_id: uint
  #   index: uint
  #   limit: uint
  # }
  def pull_request_schema() do
    {:struct, [request_id: :uint, index: :uint, limit: :uint]}
  end

  # type StreamMessage {
  #   index: uint
  #   data: data
  # }
  def message_schema() do
    {:struct, [index: :uint, data: :data]}
  end

  # type PullResponse {
  #   request_id: uint
  #   messages: []StreamMessage
  # }
  def pull_response_schema() do
    {:struct, [request_id: :uint, messages: {:array, message_schema()}]}
  end
end
