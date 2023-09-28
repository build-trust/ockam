defmodule OcklyTest do
  use ExUnit.Case
  doctest Ockly

  test "create identity" do
    {id, exported_identity} = Ockly.Native.create_identity()
    {pub_key, secret_key} = :crypto.generate_key(:eddh, :x25519)
    {pub_key2, secret_key2} = :crypto.generate_key(:eddh, :x25519)
    attestation = Ockly.Native.attest_secure_channel_key(id, secret_key)

    assert Ockly.Native.verify_secure_channel_key_attestation(
             exported_identity,
             pub_key,
             attestation
           ) == true

    # attest for another key
    assert Ockly.Native.verify_secure_channel_key_attestation(
             exported_identity,
             pub_key2,
             attestation
           ) ==
             {:error, :invalid_attestation}

    # attestation data is junk
    assert Ockly.Native.verify_secure_channel_key_attestation(exported_identity, pub_key, pub_key) ==
             {:error, :attestation_decode_error}

    assert Ockly.Native.check_identity(exported_identity) == id

    {subject_id, _subject_identity} = Ockly.Native.create_identity()
    attrs = %{"Some" => "works!", "other" => "yes!"}
    credential = Ockly.Native.issue_credential(exported_identity, subject_id, attrs, 60)

    {ttl, verified_attrs} =
      Ockly.Native.verify_credential(subject_id, [exported_identity], credential)

    {:error, :credential_verification_failed} =
      Ockly.Native.verify_credential(id, [exported_identity], credential)

    assert verified_attrs == attrs
    assert ttl == System.os_time(:second) + 60
  end

  test "create identity from existing secret" do
    {_pub, secret} = :crypto.generate_key(:eddsa, :ed25519)
    key_id = Ockly.Native.import_signing_secret(secret)
    {_id, _exported_identity} = Ockly.Native.create_identity(key_id)
  end

  test "import identity and existing secret" do
    # An existing exported identity (change history) and its signing key.
    # It must be possible to import them and use (if memory signing vault is used)

    secret =
      <<83, 231, 139, 244, 109, 254, 138, 112, 211, 93, 197, 106, 173, 226, 235, 88, 141, 218,
        113, 168, 209, 229, 28, 241, 69, 249, 106, 70, 50, 54, 218, 217>>

    identity =
      <<129, 162, 1, 88, 59, 162, 1, 1, 2, 88, 53, 164, 2, 130, 1, 129, 88, 32, 83, 241, 75, 224,
        25, 93, 231, 146, 168, 52, 2, 192, 228, 60, 198, 200, 216, 60, 101, 169, 165, 128, 75,
        221, 124, 29, 3, 224, 11, 89, 124, 70, 3, 244, 4, 26, 100, 248, 141, 178, 5, 26, 119, 196,
        144, 178, 2, 130, 1, 129, 88, 64, 236, 140, 158, 157, 188, 146, 79, 243, 149, 182, 13, 3,
        100, 174, 45, 5, 37, 208, 240, 3, 205, 7, 29, 61, 74, 44, 28, 166, 51, 161, 201, 36, 211,
        72, 21, 1, 200, 238, 124, 183, 24, 26, 236, 66, 106, 172, 219, 61, 169, 171, 103, 167, 2,
        40, 11, 183, 202, 162, 217, 237, 91, 244, 59, 1>>

    identifier = "I31f064878eb4fc0852d55a0fbb7305270b8fa1d7"

    assert identifier == Ockly.Native.check_identity(identity)
    _key_id = Ockly.Native.import_signing_secret(secret)

    # Check that we can use it
    {pub_key, secret_key} = :crypto.generate_key(:eddh, :x25519)
    attestation = Ockly.Native.attest_secure_channel_key(identifier, secret_key)

    assert Ockly.Native.verify_secure_channel_key_attestation(identity, pub_key, attestation) ==
             true
  end

  test "junk identity" do
    assert {:error, :identity_import_error} == Ockly.Native.check_identity("junk")
  end

  test "hkdf" do
    salt =
      <<122, 235, 128, 126, 98, 120, 229, 181, 70, 49, 183, 146, 114, 203, 117, 56, 57, 97, 114,
        156, 206, 162, 68, 171, 40, 228, 128, 217, 198, 93, 57, 93>>

    ikm =
      <<52, 28, 249, 202, 250, 82, 168, 196, 7, 9, 236, 217, 229, 151, 87, 163, 96, 201, 169, 224,
        128, 160, 192, 242, 238, 41, 189, 157, 200, 196, 78, 144>>

    <<k1::binary-size(32), k2::binary>> = :hkdf.derive(:sha256, ikm, "", salt, 64)

    assert k1 ==
             <<59, 23, 69, 123, 40, 228, 199, 167, 81, 220, 56, 17, 94, 81, 136, 231, 180, 67, 38,
               91, 233, 144, 215, 39, 75, 67, 179, 228, 245, 22, 187, 134>>

    assert k2 ==
             <<19, 115, 44, 135, 74, 135, 235, 12, 109, 224, 28, 81, 156, 216, 108, 224, 191, 254,
               187, 175, 111, 210, 162, 132, 249, 167, 199, 71, 188, 118, 14, 2>>
  end

  test "encrypt" do
    key =
      <<60, 39, 4, 177, 160, 228, 92, 103, 87, 110, 249, 2, 175, 175, 130, 92, 196, 211, 49, 250,
        51, 157, 6, 45, 39, 205, 207, 84, 126, 153, 104, 209>>

    plain_text = "Hello, nif"
    ad = "Token"
    nonce = 5

    {a, b} = :crypto.crypto_one_time_aead(:aes_256_gcm, key, <<nonce::96>>, plain_text, ad, true)
    cipher_text = <<a::binary, b::binary>>

    assert cipher_text ==
             <<125, 225, 184, 225, 253, 238, 233, 167, 41, 157, 48, 205, 146, 233, 209, 117, 3,
               243, 166, 199, 19, 203, 229, 132, 96, 13>>

    size = byte_size(cipher_text) - 16
    <<c::binary-size(size), tag::binary-size(16)>> = cipher_text
    decrypted = :crypto.crypto_one_time_aead(:aes_256_gcm, key, <<nonce::96>>, c, ad, tag, false)
    assert plain_text == decrypted
  end
end
