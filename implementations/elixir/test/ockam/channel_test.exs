defmodule Ockam.Channel.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Channel

  alias Ockam.Channel
  alias Ockam.Vault

  describe "Ockam.Channel" do
    @describetag :skip

    test "well known" do
      {:ok, responder_vault} = Vault.create()

      responder_s =
        to_keypair(
          responder_vault,
          "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20"
        )

      responder_e =
        to_keypair(
          responder_vault,
          "4142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f60"
        )

      {:ok, responder} =
        Channel.create(
          role: :responder,
          vault: responder_vault,
          identity_keypair: responder_s,
          e: responder_e
        )

      {:ok, initiator_vault} = Vault.create()

      initiator_s =
        to_keypair(
          initiator_vault,
          "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
        )

      initiator_e =
        to_keypair(
          initiator_vault,
          "202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f"
        )

      {:ok, _initiator} =
        Channel.create(
          role: :initiator,
          vault: initiator_vault,
          identity_keypair: initiator_s,
          e: initiator_e,
          onward_route: [responder.address]
        )
    end
  end

  def to_keypair(vault, hex) do
    bin = Base.decode16!(hex, case: :lower)
    {:ok, private_key} = Ockam.Vault.import_secret(vault, bin, type: :curve25519_private)
    {:ok, public_key} = Ockam.Vault.get_public_key(vault, private_key)
    %{private: private_key, public: public_key}
  end
end
