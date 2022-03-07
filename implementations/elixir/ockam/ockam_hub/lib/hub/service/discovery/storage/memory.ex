defmodule Ockam.Hub.Service.Discovery.Storage.Memory do
  @moduledoc """
  In-memory storage for discovery service.
  Stores registered workers as a map of %{id => ServiceInfo}
  """
  @behaviour Ockam.Hub.Service.Discovery.Storage

  alias Ockam.Hub.Service.Discovery.ServiceInfo

  @type storage_state() :: %{binary() => ServiceInfo.t()}

  def init(_options) do
    %{}
  end

  def get(id, state) do
    case Map.fetch(state, id) do
      {:ok, result} -> {{:ok, result}, state}
      :error -> {{:error, :not_found}, state}
    end
  end

  def list(state) do
    {Map.values(state), state}
  end

  def register(id, route, metadata, state) do
    ## TODO: option to override or ignore?
    new_state = Map.put(state, id, %ServiceInfo{id: id, route: route, metadata: metadata})
    {:ok, new_state}
  end
end
