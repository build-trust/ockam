defmodule Ockam.Identity.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Identity
  alias Ockam.Identity

  @existing_secret <<83, 231, 139, 244, 109, 254, 138, 112, 211, 93, 197, 106, 173, 226, 235, 88,
                     141, 218, 113, 168, 209, 229, 28, 241, 69, 249, 106, 70, 50, 54, 218, 217>>
  @existing_identity <<129, 162, 1, 88, 59, 162, 1, 1, 2, 88, 53, 164, 2, 130, 1, 129, 88, 32, 83,
                       241, 75, 224, 25, 93, 231, 146, 168, 52, 2, 192, 228, 60, 198, 200, 216,
                       60, 101, 169, 165, 128, 75, 221, 124, 29, 3, 224, 11, 89, 124, 70, 3, 244,
                       4, 26, 100, 248, 141, 178, 5, 26, 119, 196, 144, 178, 2, 130, 1, 129, 88,
                       64, 236, 140, 158, 157, 188, 146, 79, 243, 149, 182, 13, 3, 100, 174, 45,
                       5, 37, 208, 240, 3, 205, 7, 29, 61, 74, 44, 28, 166, 51, 161, 201, 36, 211,
                       72, 21, 1, 200, 238, 124, 183, 24, 26, 236, 66, 106, 172, 219, 61, 169,
                       171, 103, 167, 2, 40, 11, 183, 202, 162, 217, 237, 91, 244, 59, 1>>
  @expected_identifier "I31f064878eb4fc0852d55a0fbb7305270b8fa1d7"

  describe "Ockam.Identity.import/2" do
    test "existing identity can be imported and used" do
      {:ok, identity, identifier} = Identity.import(@existing_identity, @existing_secret)
      assert identifier == @expected_identifier
      {:ok, keypair} = Ockam.SecureChannel.Crypto.generate_dh_keypair()
      {:ok, attestation} = Identity.attest_purpose_key(identity, keypair)
      {:ok, true} = Identity.verify_purpose_key_attestation(identity, keypair.public, attestation)
    end
  end

  describe "Ockam.Identity.create/1" do
    test "identity can be created with explicit key" do
      {_pub, secret} = :crypto.generate_key(:eddsa, :ed25519)
      {:ok, identity} = Identity.create(secret)
    end
  end
end
