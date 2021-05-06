defmodule Ockam.Protocol.Stream.Partitioned.Index do
  @moduledoc false
  @behaviour Ockam.Protocol

  @impl true
  def protocol() do
    %Ockam.Protocol{
      name: "stream_index_partitioned",
      ## type Request (IndexGet | IndexSave)
      request: [get: index_get_schema(), save: index_save_schema()],
      response: index_schema()
    }
  end

  # type Index {
  #   client_id: string
  #   stream_name: string
  #   partition: uint
  #   index: optional<uint>
  # }
  def index_schema() do
    {:struct,
     [client_id: :string, stream_name: :string, partition: :uint, index: {:optional, :uint}]}
  end

  # type IndexSave {
  #   client_id: string
  #   stream_name: string
  #   partition: uint
  #   index: uint
  # }
  def index_save_schema() do
    {:struct, [client_id: :string, stream_name: :string, partition: :uint, index: :uint]}
  end

  # type IndexGet {
  #   client_id: string
  #   stream_name: string
  #   partition: uint
  # }
  def index_get_schema() do
    {:struct, [client_id: :string, stream_name: :string, partition: :uint]}
  end
end

defmodule Ockam.Protocol.Stream.Partitioned.Create do
  @moduledoc false
  @behaviour Ockam.Protocol

  @impl true
  def protocol() do
    %Ockam.Protocol{
      name: "stream_create_partitioned",
      request: create_schema(),
      ## type Response (Init | Error)
      response: init_schema()
    }
  end

  # type Init {
  #   stream_name: string
  #   partitions: uint
  # }
  def init_schema() do
    {:struct, [stream_name: :string, partition: :uint]}
  end

  # type CreateStreamRequest {
  #   stream_name: optional<string>
  #   partition: uint
  # }
  def create_schema() do
    {:struct, [stream_name: {:optional, :string}, partitions: :uint]}
  end
end
