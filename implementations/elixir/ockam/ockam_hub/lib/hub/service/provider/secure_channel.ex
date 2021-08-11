defmodule Ockam.Hub.Service.Provider.SecureChannel do
  @moduledoc """
  Implementation for Ockam.Hub.Service.Provider
  providing basic ockam routing services, :echo and :forwarding
  """

  @behaviour Ockam.Hub.Service.Provider

  @services [:secure_channel]
  @address "secure_channel_service"

  @impl true
  def services() do
    @services
  end

  @impl true
  def start_service(service_name, args) do
    options = service_options(service_name, args)
    mod = service_mod(service_name)
    mod.create(options)
  end

  @impl true
  def child_spec(service_name, args) do
    options = service_options(service_name, args)
    mod = service_mod(service_name)
    {mod, options}
  end

  def service_mod(_service), do: Ockam.SecureChannel.Listener

  def service_options(_service, _args) do
    with {:ok, vault} <- Ockam.Vault.Software.init(),
         {:ok, identity} <- Ockam.Vault.secret_generate(vault, type: :curve25519) do
      [vault: vault, identity_keypair: identity, address: @address]
    else
      error ->
        IO.puts("error starting service options for secure channel: #{inspect(error)}")
        []
    end
  rescue
    error ->
      IO.puts("error starting service options for secure channel: #{inspect(error)}")
      []
  end
end
