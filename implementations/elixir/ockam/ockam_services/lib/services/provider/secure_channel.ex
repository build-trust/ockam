defmodule Ockam.Services.Provider.SecureChannel do
  @moduledoc """
  Implementation for Ockam.Services.Provider
  providing secure channel service
  """
  @behaviour Ockam.Services.Provider

  alias Ockam.Vault.Software, as: SoftwareVault

  @services [:secure_channel, :identity_secure_channel]

  @impl true
  def services() do
    @services
  end

  @impl true
  def child_spec(:secure_channel, args) do
    options = service_options(:secure_channel, args)
    {Ockam.SecureChannel.Listener, options}
  end

  def child_spec(:identity_secure_channel, args) do
    options = service_options(:identity_secure_channel, args)
    ## TODO: make this more standard approach
    id =
      case Keyword.fetch(args, :address) do
        {:ok, address} ->
          id_str = "identity_secure_channel_" <> address
          String.to_atom(id_str)

        :error ->
          :identity_secure_channel
      end

    Supervisor.child_spec(Ockam.Identity.SecureChannel.listener_child_spec(options), %{id: id})
  end

  def service_options(:secure_channel, args) do
    with {:ok, vault} <- SoftwareVault.init(),
         {:ok, keypair} <- Ockam.Vault.secret_generate(vault, type: :curve25519) do
      Keyword.merge([vault: vault, identity_keypair: keypair, address: "secure_channel"], args)
    else
      error ->
        raise "error starting service options for secure channel: #{inspect(error)}"
    end
  end

  def service_options(:identity_secure_channel, args) do
    ## TODO: WARNING: These defaults are not for production use
    ## TODO: make it possible to read service identity from some storage
    identity_module = Keyword.get(args, :identity_module, Ockam.Identity.Stub)

    trust_policies =
      Keyword.get(args, :trust_policies, [
        {:cached_identity, [Ockam.Identity.TrustPolicy.KnownIdentitiesEts]}
      ])

    other_args = Keyword.drop(args, [:identity_module, :trust_policies])

    with {:ok, vault} <- SoftwareVault.init(),
         {:ok, keypair} <- Ockam.Vault.secret_generate(vault, type: :curve25519) do
      Keyword.merge(
        [
          identity: :dynamic,
          identity_module: identity_module,
          encryption_options: [vault: vault, identity_keypair: keypair],
          address: "identity_secure_channel",
          trust_policies: trust_policies
        ],
        other_args
      )
    else
      error ->
        raise "error starting service options for identity secure channel: #{inspect(error)}"
    end
  end
end
