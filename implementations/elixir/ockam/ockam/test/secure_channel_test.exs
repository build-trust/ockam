defmodule Ockam.SecureChannel.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.SecureChannel

  alias Ockam.SecureChannel
  alias Ockam.Vault
  alias Ockam.Vault.Software, as: SoftwareVault

  test "secure channel works" do
    secret_attributes = %{type: :curve25519, persistence: :ephemeral, length: 32}

    {:ok, responder_vault} = SoftwareVault.init()
    {:ok, responder_identity} = Vault.secret_generate(responder_vault, secret_attributes)

    {:ok, listener} =
      SecureChannel.create_listener(vault: responder_vault, identity_keypair: responder_identity)

    {:ok, initiator_vault} = SoftwareVault.init()
    {:ok, initiator_identity} = Vault.secret_generate(initiator_vault, secret_attributes)

    {:ok, _c} =
      SecureChannel.create(
        route: [listener],
        vault: initiator_vault,
        identity_keypair: initiator_identity
      )
  end
end
