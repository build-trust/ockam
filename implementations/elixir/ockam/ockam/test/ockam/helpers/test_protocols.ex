defmodule Ockam.Protocol.Tests.ExampleProtocol do
  @moduledoc false

  @behaviour Ockam.Protocol

  @impl true
  def protocol() do
    %Ockam.Protocol{
      name: "example_protocol",
      request: [structure: {:struct, [string_field: :string, int_field: :int]}, data: :data],
      response: :data
    }
  end
end

defmodule Ockam.Protocol.Mapping.Tests.Proto1 do
  @moduledoc false
  @behaviour Ockam.Protocol

  @impl true
  def protocol() do
    %Ockam.Protocol{
      name: "proto1",
      request: request(),
      response: response()
    }
  end

  def request() do
    {:struct, [data_field: :data]}
  end

  def response() do
    {:struct, [int_field: :int]}
  end
end

defmodule Ockam.Protocol.Mapping.Tests.Proto2 do
  @moduledoc false
  @behaviour Ockam.Protocol

  @impl true
  def protocol() do
    %Ockam.Protocol{
      name: "proto2",
      request: :string,
      response: :int
    }
  end
end
