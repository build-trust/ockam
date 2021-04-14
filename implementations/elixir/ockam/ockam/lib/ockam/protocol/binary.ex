defmodule Ockam.Protocol.Binary do
  @moduledoc false
  @behaviour Ockam.Protocol

  @impl true
  def protocol() do
    %Ockam.Protocol{
      name: "binary",
      request: :data
    }
  end
end
