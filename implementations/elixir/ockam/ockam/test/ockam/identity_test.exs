defmodule Ockam.Identity.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Identity
  alias Ockam.Identity

  @existing_secret <<113, 128, 116, 134, 152, 127, 151, 216, 104, 48, 23, 185, 143, 220, 176, 241,
                     25, 192, 247, 167, 161, 220, 6, 214, 165, 141, 125, 201, 237, 157, 87, 22>>
  @existing_identity <<129, 130, 88, 55, 131, 1, 1, 88, 50, 133, 246, 130, 0, 129, 88, 32, 109,
                       21, 101, 217, 183, 252, 195, 8, 12, 242, 10, 36, 37, 165, 233, 80, 220,
                       197, 241, 233, 250, 137, 65, 151, 124, 194, 152, 168, 101, 7, 48, 113, 244,
                       26, 101, 86, 63, 23, 26, 120, 34, 66, 23, 130, 0, 129, 88, 64, 117, 93,
                       149, 142, 37, 229, 76, 223, 234, 124, 175, 116, 136, 206, 48, 67, 95, 15,
                       226, 107, 78, 127, 9, 140, 165, 1, 50, 122, 246, 176, 76, 158, 45, 196,
                       151, 216, 148, 237, 113, 222, 72, 162, 7, 59, 126, 108, 122, 209, 121, 133,
                       147, 62, 138, 249, 186, 146, 249, 74, 88, 173, 176, 136, 148, 10>>
  @expected_identifier "I10253701dafcc65a621ad9fb4097cb327c541de78827713320b749cbbdbd2e9f"

  describe "Ockam.Identity.import/2" do
    test "existing identity can be imported and used" do
      {:ok, identity, identifier} = Identity.import(@existing_identity, @existing_secret)
      assert Ockam.Identity.Identifier.to_str(identifier) == @expected_identifier
      {:ok, keypair} = Ockam.SecureChannel.Crypto.generate_dh_keypair()
      {:ok, attestation} = Identity.attest_purpose_key(identity, keypair)
      {:ok, true} = Identity.verify_purpose_key_attestation(identity, keypair.public, attestation)
    end
  end

  describe "Ockam.Identity.create/1" do
    test "identity can be created with explicit key" do
      {_pub, secret} = :crypto.generate_key(:eddsa, :ed25519)
      {:ok, _identity} = Identity.create(secret)
    end
  end
end
