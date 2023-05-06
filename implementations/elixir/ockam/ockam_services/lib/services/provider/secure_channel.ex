defmodule Ockam.Services.Provider.SecureChannel do
  @moduledoc """
  Implementation for Ockam.Services.Provider
  providing secure channel service
  """
  @behaviour Ockam.Services.Provider

  alias Ockam.Vault.Software, as: SoftwareVault

  @services [:secure_channel]

  @impl true
  def services() do
    @services
  end

  @impl true
  def child_spec(:secure_channel, args) do
    options = service_options(:secure_channel, args)
    ## TODO: make this more standard approach
    id =
      case Keyword.fetch(args, :address) do
        {:ok, address} ->
          id_str = "secure_channel_" <> address
          String.to_atom(id_str)

        :error ->
          :secure_channel
      end

    Supervisor.child_spec(Ockam.SecureChannel.Channel.listener_child_spec(options), %{id: id})
  end

  def service_options(:secure_channel, args) do
    ## TODO: make it possible to read service identity from some storage
    identity_module = Keyword.get(args, :identity_module, Ockam.Identity.default_implementation())

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
          encryption_options: [vault: vault, static_keypair: keypair],
          address: "secure_channel",
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
