defmodule Ockam.Services.Provider.SecureChannel do
  @moduledoc """
  Implementation for Ockam.Services.Provider
  providing secure channel service
  """
  @behaviour Ockam.Services.Provider

  alias Ockam.Identity
  alias Ockam.SecureChannel.Crypto


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
    trust_policies =
      Keyword.get(args, :trust_policies, [
        {:cached_identity, [Ockam.Identity.TrustPolicy.KnownIdentitiesEts]}
      ])

    other_args = Keyword.drop(args, [:trust_policies])


    with {:ok, identity} <- Identity.create(),
         {:ok, keypair} <- Crypto.generate_dh_keypair(),
         {:ok, attestation} <- Identity.attest_purpose_key(identity, keypair.public) do
      Keyword.merge(
        [
	  identity: identity,
          encryption_options: [static_keypair: keypair, static_key_attestation: attestation],
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
