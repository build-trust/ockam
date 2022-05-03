defmodule Ockam.Services.Provider.Routing do
  @moduledoc """
  Implementation for Ockam.Services.Provider
  providing basic ockam routing services, :echo and :forwarding
  """

  @behaviour Ockam.Services.Provider

  alias Ockam.Services.Echo, as: EchoService
  alias Ockam.Services.Forwarding, as: ForwardingService
  alias Ockam.Services.PubSub, as: PubSubService
  alias Ockam.Services.StaticForwarding, as: StaticForwardingService
  alias Ockam.Services.Tracing, as: TracingService

  ## TODO: API to start all services in a provider?
  @services [:echo, :forwarding, :static_forwarding, :pub_sub, :tracing]

  @impl true
  def services() do
    @services
  end

  @impl true
  def child_spec(:echo, args) do
    {EchoService, Keyword.merge([address: "echo"], args)}
  end

  def child_spec(:forwarding, args) do
    {ForwardingService,
     Keyword.merge([address: "forwarding", extra_addresses: ["forwarding_service"]], args)}
  end

  def child_spec(:static_forwarding, args) do
    {StaticForwardingService,
     Keyword.merge(
       [
         address: "static_forwarding",
         prefix: "forward_to",
         extra_addresses: ["static_forwarding_service"]
       ],
       args
     )}
  end

  def child_spec(:pub_sub, args) do
    {PubSubService, Keyword.merge([address: "pub_sub", prefix: "pub_sub_t"], args)}
  end

  def child_spec(:tracing, args) do
    {TracingService, Keyword.merge([address: "tracing"], args)}
  end
end
