defmodule Ockam.Hub.Service.Provider.Discovery do
  @moduledoc """
  Implementation for Ockam.Hub.Service.Provider
  providing discovery service
  """

  @behaviour Ockam.Hub.Service.Provider

  alias Ockam.Hub.Service.Discovery, as: DiscoveryService

  @services [:discovery]

  @impl true
  def services() do
    @services
  end

  @impl true
  def child_spec(:discovery, args) do
    {DiscoveryService,
     Keyword.merge(
       [
         address: "discovery",
         storage: Ockam.Hub.Service.Discovery.Storage.Supervisor,
         ## TODO: provide superviser from args
         storage_options: [supervisor: Ockam.Hub.Service.Provider]
       ],
       args
     )}
  end
end
