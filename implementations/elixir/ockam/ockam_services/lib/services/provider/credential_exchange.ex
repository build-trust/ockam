defmodule Ockam.Services.Provider.CredentialExchange do
  @moduledoc """
  Implementation for Ockam.Services.Provider
  providing asymmetrical credential exchange api service
  """

  @behaviour Ockam.Services.Provider

  alias Ockam.Services.API.CredentialExchange

  @services [:credential_exchange]

  @impl true
  def services() do
    @services
  end

  @impl true
  def child_spec(:credential_exchange, args) do
    {CredentialExchange,
     Keyword.merge(
       [
         address: "credential_exchange",
         authorities: []
       ],
       args
     )}
  end
end
