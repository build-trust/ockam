defmodule Ockam.Services.Provider.ABAC.PoliciesApi do
  @moduledoc """
  Implementation for Ockam.Services.Provider
  providing ABAC policies setting/retrieval API
  """

  @behaviour Ockam.Services.Provider

  alias Ockam.Services.API.ABAC.PoliciesApi

  @services [:abac_policies]

  @impl true
  def services() do
    @services
  end

  @impl true
  def child_spec(:abac_policies, args) do
    {PoliciesApi,
     Keyword.merge(
       [
         address: "abac_policies"
       ],
       args
     )}
  end
end
