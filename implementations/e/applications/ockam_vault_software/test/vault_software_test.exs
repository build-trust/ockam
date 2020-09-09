defmodule Ockam.Vault.Software.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Vault.Software

  describe "Ockam.Vault.Software.sha256/2" do
    test "can run natively implemented functions" do
      {:ok, handle} = Ockam.Vault.Software.default_init
      {:ok, hash} = Ockam.Vault.Software.sha256(handle, "test")
      assert hash == <<159, 134, 208, 129, 136, 76, 125, 101, 154,
                       47, 234, 160, 197, 90, 208, 21, 163, 191,
                       79, 27, 43, 11, 130, 44, 209, 93, 108,
                       21, 176, 240, 10, 8>>
    end
  end
  describe "Ockam.Vault.Software.random_bytes/2" do
    test "can run natively implemented functions" do
      {:ok, handle} = Ockam.Vault.Software.default_init
      {:ok, random1} = Ockam.Vault.Software.random_bytes(handle, 32)
      assert random1 != nil
      assert byte_size(random1) == 32
      {:ok, random2} = Ockam.Vault.Software.random_bytes(handle, 32)
      assert random2 != nil
      assert byte_size(random2) == 32
      assert random1 != random2
    end
  end

  describe "Ockam.Vault.Software.secret_generate/2" do
    test "can run natively implemented functions" do
      {:ok, handle} = Ockam.Vault.Software.default_init
      attributes = %{type: :curve25519, persistence: :ephemeral, purpose: :key_agreement}
      {:ok, secret} = Ockam.Vault.Software.secret_generate(handle, attributes)
      assert secret != 0
    end
  end

  describe "Ockam.Vault.Software.secret_import/3" do
    test "can run natively implemented functions" do
      {:ok, handle} = Ockam.Vault.Software.default_init
      attributes = %{type: :curve25519, persistence: :ephemeral, purpose: :key_agreement}
      key_data   = <<120, 132, 203, 140, 22, 250, 109, 249,
                     155, 207, 102, 47, 186, 14, 109, 252,
                     110, 197, 217, 163, 147, 242, 36, 234,
                     91, 58, 252, 218, 244, 55, 133, 86>>

      {:ok, secret} = Ockam.Vault.Software.secret_import(handle, attributes, key_data)
      assert secret != 0
    end
  end

  describe "Ockam.Vault.Software.secret_export/2" do
    test "can run natively implemented functions" do
      {:ok, handle} = Ockam.Vault.Software.default_init
      attributes = %{type: :curve25519, persistence: :ephemeral, purpose: :key_agreement}
      key_data   = <<120, 132, 203, 140, 22, 250, 109, 249,
                     155, 207, 102, 47, 186, 14, 109, 252,
                     110, 197, 217, 163, 147, 242, 36, 234,
                     91, 58, 252, 218, 244, 55, 133, 86>>

      {:ok, secret} = Ockam.Vault.Software.secret_import(handle, attributes, key_data)
      

      {:ok, data} = Ockam.Vault.Software.secret_export(handle, secret)

      assert data == key_data
    end
  end

  describe "Ockam.Vault.Software.secret_publickey_get/2" do
    test "can run natively implemented functions" do
      {:ok, handle} = Ockam.Vault.Software.default_init
      attributes = %{type: :curve25519, persistence: :ephemeral, purpose: :key_agreement}
      key_data   = <<120, 132, 203, 140, 22, 250, 109, 249,
                     155, 207, 102, 47, 186, 14, 109, 252,
                     110, 197, 217, 163, 147, 242, 36, 234,
                     91, 58, 252, 218, 244, 55, 133, 86>>

      {:ok, secret} = Ockam.Vault.Software.secret_import(handle, attributes, key_data)

      public_key = <<150, 222, 161, 134, 252, 228, 164, 141,
                     155, 94, 150, 20, 255, 187, 168, 204,
                     82, 148, 227, 235, 101, 45, 106, 171,
                     61, 223, 40, 223, 225, 181, 77, 102>>

      {:ok, data} = Ockam.Vault.Software.secret_publickey_get(handle, secret)

      assert data == public_key
    end
  end

  describe "Ockam.Vault.Software.secret_attributes_get/2" do
    test "can run natively implemented functions" do
      {:ok, handle} = Ockam.Vault.Software.default_init
      attributes = %{type: :curve25519, persistence: :ephemeral, purpose: :key_agreement}
      {:ok, secret} = Ockam.Vault.Software.secret_generate(handle, attributes)

      {:ok, attributes} = Ockam.Vault.Software.secret_attributes_get(handle, secret)

      assert attributes == %{type: :curve25519, persistence: :ephemeral, purpose: :key_agreement}
    end
  end

  describe "Ockam.Vault.Software.secret_destroy/2" do
    test "can run natively implemented functions" do
      {:ok, handle} = Ockam.Vault.Software.default_init
      attributes = %{type: :curve25519, persistence: :ephemeral, purpose: :key_agreement}
      {:ok, secret} = Ockam.Vault.Software.secret_generate(handle, attributes)

      :ok = Ockam.Vault.Software.secret_destroy(handle, secret)
    end
  end

  describe "Ockam.Vault.Software.ecdh/3" do
    test "can run natively implemented functions" do
      {:ok, handle} = Ockam.Vault.Software.default_init
      attributes = %{type: :curve25519, persistence: :ephemeral, purpose: :key_agreement}

      secret_data = <<136, 150, 7, 173, 189, 63, 35, 127,
                      17, 37, 185, 84, 167, 243, 90, 61,
                      140, 73, 183, 46, 177, 139, 20, 171,
                      175, 41, 171, 202, 146, 55, 186, 114>>

      {:ok, secret1} = Ockam.Vault.Software.secret_import(handle, attributes, secret_data)

      public2 = <<244, 220, 38, 193, 253, 60, 127, 20,
                  18, 61, 120, 162, 140, 188, 230, 36,
                  20, 82, 31, 186, 20, 207, 112, 14,
                  88, 119, 23, 20, 119, 179, 226, 95>>

      {:ok, dh} = Ockam.Vault.Software.ecdh(handle, secret1, public2)

      {:ok, dh_data} = Ockam.Vault.Software.secret_export(handle, dh)
      
      assert dh_data == <<174, 139, 240, 140, 226, 187, 236, 169,
                          59, 89, 38, 171, 165, 29, 32, 47,
                          148, 161, 218, 139, 246, 23, 131, 164,
                          6, 109, 155, 8, 203, 90, 153, 38>>
    end
  end

  describe "Ockam.Vault.Software.hkdf_sha256/4" do
    test "can run natively implemented functions" do
      {:ok, handle} = Ockam.Vault.Software.default_init
      attributes = %{type: :buffer, persistence: :ephemeral, purpose: :key_agreement, length: 32}

      salt_data = <<122, 235, 128, 126, 98, 120, 229, 181,
                    70, 49, 183, 146, 114, 203, 117, 56,
                    57, 97, 114, 156, 206, 162, 68, 171,
                    40, 228, 128, 217, 198, 93, 57, 93>>
      {:ok, salt} = Ockam.Vault.Software.secret_import(handle, attributes, salt_data)

      ikm_data = <<52, 28, 249, 202, 250, 82, 168, 196,
                   7, 9, 236, 217, 229, 151, 87, 163,
                   96, 201, 169, 224, 128, 160, 192, 242,
                   238, 41, 189, 157, 200, 196, 78, 144>>
      {:ok, ikm} = Ockam.Vault.Software.secret_import(handle, attributes, ikm_data)

      {:ok, derived_secrets} = Ockam.Vault.Software.hkdf_sha256(handle, salt, ikm, 2)

      {:ok, data1} = Ockam.Vault.Software.secret_export(handle, Enum.at(derived_secrets, 0))
      {:ok, data2} = Ockam.Vault.Software.secret_export(handle, Enum.at(derived_secrets, 1))

      assert data1 == <<59, 23, 69, 123, 40, 228, 199, 167,
                        81, 220, 56, 17, 94, 81, 136, 231,
                        180, 67, 38, 91, 233, 144, 215, 39,
                        75, 67, 179, 228, 245, 22, 187, 134>>

      assert data2 == <<19, 115, 44, 135, 74, 135, 235, 12,
                        109, 224, 28, 81, 156, 216, 108, 224,
                        191, 254, 187, 175, 111, 210, 162, 132,
                        249, 167, 199, 71, 188, 118, 14, 2>>
    end
  end
end
