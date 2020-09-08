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
end
