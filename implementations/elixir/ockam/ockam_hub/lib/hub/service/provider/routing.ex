defmodule Ockam.Hub.Service.Provider.Routing do
  @moduledoc """
  Implementation for Ockam.Hub.Service.Provider
  providing basic ockam routing services, :echo and :forwarding
  """

  @behaviour Ockam.Hub.Service.Provider

  alias Ockam.Hub.Service.Echo, as: EchoService
  alias Ockam.Hub.Service.Forwarding, as: ForwardingService
  alias Ockam.Hub.Service.PubSub, as: PubSubService
  alias Ockam.Hub.Service.Tracing, as: TracingService

  @services [:echo, :forwarding, :pub_sub, :tracing]

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
    ForwardingService.create(Keyword.merge([address: "forwarding_service"], args))
  end

  def start_service(:pub_sub, args) do
    PubSubService.create(Keyword.merge([address: "pub_sub_service", prefix: "pub_sub_t"], args))
  end

  def start_service(:tracing, args) do
    TracingService.create(Keyword.merge([address: "tracing_service"], args))
  end

  @impl true
  def child_spec(:echo, args) do
    {EchoService, Keyword.merge([address: "echo_service"], args)}
  end

  def child_spec(:forwarding, args) do
    {ForwardingService, Keyword.merge([address: "forwarding_service"], args)}
  end

  def child_spec(:pub_sub, args) do
    {PubSubService, Keyword.merge([address: "pub_sub_service", prefix: "pub_sub_t"], args)}
  end

  def child_spec(:tracing, args) do
    {TracingService, Keyword.merge([address: "tracing_service"], args)}
  end
end
