defmodule Ockam.Vault.Software.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Vault.Software
  alias Ockam.Vault.Software, as: SoftwareVault

  describe "Ockam.Vault.Software.sha256/2" do
    test "can run natively implemented functions" do
      {:ok, handle} = SoftwareVault.default_init()
      {:ok, hash} = SoftwareVault.sha256(handle, "test")

      assert hash ==
               <<159, 134, 208, 129, 136, 76, 125, 101, 154, 47, 234, 160, 197, 90, 208, 21, 163,
                 191, 79, 27, 43, 11, 130, 44, 209, 93, 108, 21, 176, 240, 10, 8>>
    end
  end

  describe "Ockam.Vault.Software.file_init/1" do
    test "can run natively implemented functions" do
      temp_dir = System.tmp_dir!()
      vault_dir = Path.join(temp_dir, "a")

      {:ok, handle} = SoftwareVault.file_init(vault_dir)
      {:ok, hash} = SoftwareVault.sha256(handle, "test")

      assert hash ==
               <<159, 134, 208, 129, 136, 76, 125, 101, 154, 47, 234, 160, 197, 90, 208, 21, 163,
                 191, 79, 27, 43, 11, 130, 44, 209, 93, 108, 21, 176, 240, 10, 8>>

      File.rm_rf!(vault_dir)
    end
  end

  describe "Ockam.Vault.Software.secret_generate_persistence/2" do
    test "can run natively implemented functions" do
      temp_dir = System.tmp_dir!()
      vault_dir = Path.join(temp_dir, "a")

      {:ok, handle} = SoftwareVault.file_init(vault_dir)
      attributes = %{type: :curve25519, persistence: :persistent, length: 32}
      {:ok, secret} = SoftwareVault.secret_generate(handle, attributes)
      assert secret != 0
      {:ok, data1} = SoftwareVault.secret_export(handle, secret)

      {:ok, persistence_id} = SoftwareVault.get_persistence_id(handle, secret)

      SoftwareVault.deinit(handle)
      {:ok, handle2} = SoftwareVault.file_init(vault_dir)
      {:ok, secret2} = SoftwareVault.get_persistent_secret(handle2, persistence_id)
      {:ok, data} = SoftwareVault.secret_export(handle2, secret2)
      assert data == data1

      File.rm_rf!(vault_dir)
    end
  end

  describe "Ockam.Vault.Software.secret_generate/2" do
    test "can run natively implemented functions" do
      {:ok, handle} = SoftwareVault.default_init()
      attributes = %{type: :curve25519, persistence: :ephemeral, length: 32}
      {:ok, secret} = SoftwareVault.secret_generate(handle, attributes)
      assert secret != 0
    end
  end

  describe "Ockam.Vault.Software.secret_import/3" do
    test "can run natively implemented functions" do
      {:ok, handle} = SoftwareVault.default_init()
      attributes = %{type: :curve25519, persistence: :ephemeral, length: 32}

      key_data =
        <<120, 132, 203, 140, 22, 250, 109, 249, 155, 207, 102, 47, 186, 14, 109, 252, 110, 197,
          217, 163, 147, 242, 36, 234, 91, 58, 252, 218, 244, 55, 133, 86>>

      {:ok, secret} = SoftwareVault.secret_import(handle, attributes, key_data)
      assert secret != 0
    end
  end

  describe "Ockam.Vault.Software.secret_export/2" do
    test "can run natively implemented functions" do
      {:ok, handle} = SoftwareVault.default_init()
      attributes = %{type: :curve25519, persistence: :ephemeral, length: 32}

      key_data =
        <<120, 132, 203, 140, 22, 250, 109, 249, 155, 207, 102, 47, 186, 14, 109, 252, 110, 197,
          217, 163, 147, 242, 36, 234, 91, 58, 252, 218, 244, 55, 133, 86>>

      {:ok, secret} = SoftwareVault.secret_import(handle, attributes, key_data)

      {:ok, data} = SoftwareVault.secret_export(handle, secret)

      assert data == key_data
    end
  end

  describe "Ockam.Vault.Software.secret_publickey_get/2" do
    test "can run natively implemented functions" do
      {:ok, handle} = SoftwareVault.default_init()
      attributes = %{type: :curve25519, persistence: :ephemeral, length: 32}

      key_data =
        <<120, 132, 203, 140, 22, 250, 109, 249, 155, 207, 102, 47, 186, 14, 109, 252, 110, 197,
          217, 163, 147, 242, 36, 234, 91, 58, 252, 218, 244, 55, 133, 86>>

      {:ok, secret} = SoftwareVault.secret_import(handle, attributes, key_data)

      public_key =
        <<150, 222, 161, 134, 252, 228, 164, 141, 155, 94, 150, 20, 255, 187, 168, 204, 82, 148,
          227, 235, 101, 45, 106, 171, 61, 223, 40, 223, 225, 181, 77, 102>>

      {:ok, data} = SoftwareVault.secret_publickey_get(handle, secret)

      assert data == public_key
    end
  end

  describe "Ockam.Vault.Software.secret_attributes_get/2" do
    test "can run natively implemented functions" do
      {:ok, handle} = SoftwareVault.default_init()
      attributes = %{type: :curve25519, persistence: :ephemeral, length: 32}
      {:ok, secret} = SoftwareVault.secret_generate(handle, attributes)

      {:ok, attributes} = SoftwareVault.secret_attributes_get(handle, secret)

      assert attributes == %{type: :curve25519, persistence: :ephemeral, length: 32}
    end
  end

  describe "Ockam.Vault.Software.secret_destroy/2" do
    test "can run natively implemented functions" do
      {:ok, handle} = SoftwareVault.default_init()
      attributes = %{type: :curve25519, persistence: :ephemeral, length: 32}
      {:ok, secret} = SoftwareVault.secret_generate(handle, attributes)

      :ok = SoftwareVault.secret_destroy(handle, secret)
    end
  end

  describe "Ockam.Vault.Software.ecdh/3" do
    test "can run natively implemented functions" do
      {:ok, handle} = SoftwareVault.default_init()
      attributes = %{type: :curve25519, persistence: :ephemeral, length: 32}

      secret_data =
        <<136, 150, 7, 173, 189, 63, 35, 127, 17, 37, 185, 84, 167, 243, 90, 61, 140, 73, 183, 46,
          177, 139, 20, 171, 175, 41, 171, 202, 146, 55, 186, 114>>

      {:ok, secret1} = SoftwareVault.secret_import(handle, attributes, secret_data)

      public2 =
        <<244, 220, 38, 193, 253, 60, 127, 20, 18, 61, 120, 162, 140, 188, 230, 36, 20, 82, 31,
          186, 20, 207, 112, 14, 88, 119, 23, 20, 119, 179, 226, 95>>

      {:ok, dh} = SoftwareVault.ecdh(handle, secret1, public2)

      {:ok, dh_data} = SoftwareVault.secret_export(handle, dh)

      assert dh_data ==
               <<174, 139, 240, 140, 226, 187, 236, 169, 59, 89, 38, 171, 165, 29, 32, 47, 148,
                 161, 218, 139, 246, 23, 131, 164, 6, 109, 155, 8, 203, 90, 153, 38>>
    end
  end

  describe "Ockam.Vault.Software.hkdf_sha256/4" do
    test "can run natively implemented functions" do
      {:ok, handle} = SoftwareVault.default_init()
      attributes = %{type: :buffer, persistence: :ephemeral, length: 32}

      salt_data =
        <<122, 235, 128, 126, 98, 120, 229, 181, 70, 49, 183, 146, 114, 203, 117, 56, 57, 97, 114,
          156, 206, 162, 68, 171, 40, 228, 128, 217, 198, 93, 57, 93>>

      {:ok, salt} = SoftwareVault.secret_import(handle, attributes, salt_data)

      ikm_data =
        <<52, 28, 249, 202, 250, 82, 168, 196, 7, 9, 236, 217, 229, 151, 87, 163, 96, 201, 169,
          224, 128, 160, 192, 242, 238, 41, 189, 157, 200, 196, 78, 144>>

      {:ok, ikm} = SoftwareVault.secret_import(handle, attributes, ikm_data)

      attributes_out1 = %{
        type: :buffer,
        persistence: :ephemeral,
        length: 32
      }

      attributes_out2 = %{
        type: :buffer,
        persistence: :ephemeral,
        length: 32
      }

      {:ok, derived_secrets} =
        SoftwareVault.hkdf_sha256(handle, salt, ikm, [attributes_out1, attributes_out2])

      {:ok, data1} = SoftwareVault.secret_export(handle, Enum.at(derived_secrets, 0))
      {:ok, data2} = SoftwareVault.secret_export(handle, Enum.at(derived_secrets, 1))

      assert data1 ==
               <<59, 23, 69, 123, 40, 228, 199, 167, 81, 220, 56, 17, 94, 81, 136, 231, 180, 67,
                 38, 91, 233, 144, 215, 39, 75, 67, 179, 228, 245, 22, 187, 134>>

      assert data2 ==
               <<19, 115, 44, 135, 74, 135, 235, 12, 109, 224, 28, 81, 156, 216, 108, 224, 191,
                 254, 187, 175, 111, 210, 162, 132, 249, 167, 199, 71, 188, 118, 14, 2>>
    end
  end

  describe "Ockam.Vault.Software.aead_aes_gcm_encrypt/5" do
    test "can run natively implemented functions" do
      {:ok, handle} = SoftwareVault.default_init()
      attributes = %{type: :aes, persistence: :ephemeral, length: 32}

      key_data =
        <<60, 39, 4, 177, 160, 228, 92, 103, 87, 110, 249, 2, 175, 175, 130, 92, 196, 211, 49,
          250, 51, 157, 6, 45, 39, 205, 207, 84, 126, 153, 104, 209>>

      {:ok, key} = SoftwareVault.secret_import(handle, attributes, key_data)

      plain_text = "Hello, nif"
      ad = "Token"
      nonce = 5

      {:ok, cipher_text} = SoftwareVault.aead_aes_gcm_encrypt(handle, key, nonce, ad, plain_text)

      assert cipher_text ==
               <<125, 225, 184, 225, 253, 238, 233, 167, 41, 157, 48, 205, 146, 233, 209, 117, 3,
                 243, 166, 199, 19, 203, 229, 132, 96, 13>>
    end
  end

  describe "Ockam.Vault.Software.aead_aes_gcm_decrypt/5" do
    test "can run natively implemented functions" do
      {:ok, handle} = SoftwareVault.default_init()
      attributes = %{type: :aes, persistence: :ephemeral, length: 32}

      key_data =
        <<60, 39, 4, 177, 160, 228, 92, 103, 87, 110, 249, 2, 175, 175, 130, 92, 196, 211, 49,
          250, 51, 157, 6, 45, 39, 205, 207, 84, 126, 153, 104, 209>>

      {:ok, key} = SoftwareVault.secret_import(handle, attributes, key_data)

      plain_text = "Hello, nif"
      ad = "Token"
      nonce = 5

      cipher_text =
        <<125, 225, 184, 225, 253, 238, 233, 167, 41, 157, 48, 205, 146, 233, 209, 117, 3, 243,
          166, 199, 19, 203, 229, 132, 96, 13>>

      {:ok, decrypted} = SoftwareVault.aead_aes_gcm_decrypt(handle, key, nonce, ad, cipher_text)

      assert plain_text == decrypted
    end
  end

  describe "Ockam.Vault.Software.aead_aes_gcm_encrypt_decrypt/5" do
    test "can run natively implemented functions" do
      {:ok, handle} = SoftwareVault.default_init()
      attributes = %{type: :aes, persistence: :ephemeral, length: 32}

      {:ok, key} = SoftwareVault.secret_generate(handle, attributes)

      plain_text = "Hello, nif"
      ad = "Token"
      nonce = 5

      {:ok, cipher_text} = SoftwareVault.aead_aes_gcm_encrypt(handle, key, nonce, ad, plain_text)

      {:ok, decrypted} = SoftwareVault.aead_aes_gcm_decrypt(handle, key, nonce, ad, cipher_text)

      assert plain_text == decrypted
    end
  end

  describe "Ockam.Vault.Software.deinit/1" do
    test "can run natively implemented functions" do
      {:ok, handle} = SoftwareVault.default_init()

      :ok = SoftwareVault.deinit(handle)
    end
  end

  describe "Ockam.Vault.Software.xx" do
    test "can run natively implemented functions" do
      {:ok, vault_handle} = SoftwareVault.default_init()

      attributes = %{type: :curve25519, persistence: :persistent, length: 32}

      {:ok, initiator_key} = SoftwareVault.secret_generate(vault_handle, attributes)
      {:ok, initiator_pub} = SoftwareVault.secret_publickey_get(vault_handle, initiator_key)

      {:ok, responder_key} = SoftwareVault.secret_generate(vault_handle, attributes)
      {:ok, responder_pub} = SoftwareVault.secret_publickey_get(vault_handle, responder_key)

      {:ok, initiator_handle} = SoftwareVault.xx_initiator(vault_handle, initiator_key)
      {:ok, is_somplete_initiator1} = SoftwareVault.is_complete(initiator_handle)
      assert is_somplete_initiator1 == 0

      {:ok, responder_handle} = SoftwareVault.xx_responder(vault_handle, responder_key)
      {:ok, is_somplete_responder1} = SoftwareVault.is_complete(responder_handle)
      assert is_somplete_responder1 == 0

      {:ok, response1} = SoftwareVault.process(initiator_handle, <<>>)
      {:ok, is_somplete_initiator2} = SoftwareVault.is_complete(initiator_handle)
      assert is_somplete_initiator2 == 0

      {:ok, payload1} = SoftwareVault.process(responder_handle, response1)
      assert payload1 == <<>>
      {:ok, is_somplete_responder2} = SoftwareVault.is_complete(responder_handle)
      assert is_somplete_responder2 == 0

      {:ok, response2} = SoftwareVault.process(responder_handle, <<>>)
      {:ok, is_somplete_responder3} = SoftwareVault.is_complete(responder_handle)
      assert is_somplete_responder3 == 0

      {:ok, payload2} = SoftwareVault.process(initiator_handle, response2)
      assert payload2 == <<>>

      {:ok, is_somplete_initiator3} = SoftwareVault.is_complete(initiator_handle)
      assert is_somplete_initiator3 == 0

      {:ok, response3} = SoftwareVault.process(initiator_handle, <<>>)
      {:ok, is_somplete_initiator4} = SoftwareVault.is_complete(initiator_handle)
      assert is_somplete_initiator4 == 1

      {:ok, payload3} = SoftwareVault.process(responder_handle, response3)
      assert payload3 == <<>>
      {:ok, is_somplete_responder4} = SoftwareVault.is_complete(responder_handle)
      assert is_somplete_responder4 == 1

      {:ok, initiator} = SoftwareVault.finalize(initiator_handle)
      {:ok, responder} = SoftwareVault.finalize(responder_handle)

      assert initiator.h == responder.h
      assert initiator.public_key == responder_pub
      assert responder.public_key == initiator_pub

      text1 = "Hello, nif"
      text2 = "Hello, rust"

      {:ok, cipher_text1} =
        SoftwareVault.aead_aes_gcm_encrypt(vault_handle, initiator.encrypt_key, 5, <<>>, text1)

      {:ok, plain_text1} =
        SoftwareVault.aead_aes_gcm_decrypt(
          vault_handle,
          responder.decrypt_key,
          5,
          <<>>,
          cipher_text1
        )

      assert plain_text1 == text1

      {:ok, cipher_text2} =
        SoftwareVault.aead_aes_gcm_encrypt(vault_handle, responder.encrypt_key, 6, <<>>, text2)

      {:ok, plain_text2} =
        SoftwareVault.aead_aes_gcm_decrypt(
          vault_handle,
          initiator.decrypt_key,
          6,
          <<>>,
          cipher_text2
        )

      assert plain_text2 == text2
    end
  end
end
