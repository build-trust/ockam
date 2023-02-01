defmodule Ockam.Services.TokenLeaseManager.Provider do
  @moduledoc false
  @behaviour Ockam.Services.Provider

  @services [:influxdb_token_lease]
  @address "influxdb_token_lease"

  @impl true
  def services() do
    @services
  end

  @impl true
  def child_spec(service_name, args) do
    options = service_options(service_name, args)
    mod = service_mod(service_name)
    {mod, options}
  end

  def service_mod(_service) do
    Ockam.Services.TokenLeaseManager
  end

  def service_options(_service, args), do: [address: @address] ++ args
end
