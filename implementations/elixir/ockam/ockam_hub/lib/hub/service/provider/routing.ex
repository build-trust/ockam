defmodule Ockam.Hub.Service.Provider.Routing do
  @moduledoc """
  Implementation for Ockam.Hub.Service.Provider
  providing basic ockam routing services, :echo and :forwarding
  """

  @behaviour Ockam.Hub.Service.Provider

  alias Ockam.Hub.Service.Echo, as: EchoService
  alias Ockam.Hub.Service.Forwarding, as: ForwardingService
  alias Ockam.Hub.Service.Alias, as: AliasService

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
    alias_result = AliasService.create(Keyword.merge([address: "alias_service"], args))
    forwarding_result = ForwardingService.create(Keyword.merge([address: "forwarding_service"], args))

    case {forwarding_result, alias_result} do
      {{:ok, fr}, {:ok, ar}} ->
        {:ok, {fr, ar}}
      {fr, ar} ->
        {:error, [forwarding: fr, alias: ar]}
    end
  end
end
