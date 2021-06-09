defmodule Ockam.Hub.Service.Provider.Routing do
  @moduledoc """
  Implementation for Ockam.Hub.Service.Provider
  providing basic ockam routing services, :echo and :forwarding
  """

  @behaviour Ockam.Hub.Service.Provider

  alias Ockam.Hub.Service.Alias, as: AliasService
  alias Ockam.Hub.Service.Echo, as: EchoService

  @services [:echo, :forwarding]

  @impl true
  def services() do
    @services
  end

  @impl true
  def start_service(:echo, args) do
    ## TODO: start services as permanent
    EchoService.create(Keyword.merge([address: "echo_service"], args))
  end

  def start_service(:forwarding, args) do
    AliasService.create(Keyword.merge([address: "forwarding_service"], args))
  end
end
