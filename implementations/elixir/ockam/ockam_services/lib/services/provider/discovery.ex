defmodule Ockam.Services.Provider.Discovery do
  @moduledoc """
  Implementation for Ockam.Services.Provider
  providing discovery service
  """

  @behaviour Ockam.Services.Provider

  alias Ockam.Services.API.Discovery, as: DiscoveryService

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
         storage: Ockam.Services.Discovery.Storage.Supervisor,
         ## TODO: provide superviser from args
         storage_options: [supervisor: Ockam.Services.Provider]
       ],
       args
     )}
  end
end
