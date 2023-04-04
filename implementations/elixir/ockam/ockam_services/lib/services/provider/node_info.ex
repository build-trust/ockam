defmodule Ockam.Services.Provider.NodeInfo do
  @moduledoc """
  Implementation for Ockam.Services.Provider
  providing node info service
  """

  @behaviour Ockam.Services.Provider

  alias Ockam.Services.API.NodeInfo

  @services [:node_info]

  @impl true
  def services() do
    @services
  end

  @impl true
  def child_spec(:node_info, args) do
    {NodeInfo, Keyword.merge([address: "node_info"], args)}
  end
end
