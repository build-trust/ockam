defmodule Ockam.Vault.Tests do
  use ExUnit.Case, async: true

  alias Ockam.Vault
  alias Ockam.Vault.KeyPair
  alias Ockam.Vault.SecretAttributes
  alias Ockam.Test.Fixtures

  setup do
    {:ok, vault} = Vault.new()
    {:ok, [vault: vault]}
  end

  test "curve25519", %{vault: vault} do
    attrs = SecretAttributes.x25519(:ephemeral)
    kp1 = KeyPair.new(vault, attrs)
    kp2 = KeyPair.new(vault, attrs)

    {:ok, a} = Vault.ecdh(vault, kp1, kp2)
    {:ok, b} = Vault.ecdh(vault, kp2, kp1)
    {:ok, a_bin} = Vault.export_secret(vault, a)
    {:ok, b_bin} = Vault.export_secret(vault, b)
    assert ^a_bin = b_bin

    %{a_pub: a_pub, a_priv: a_priv, b_pub: b_pub, b_priv: b_priv, shared: shared} =
      Fixtures.curve25519_data()

    kp3 = KeyPair.new(vault, private: a_priv, public: a_pub, attrs: attrs)
    kp4 = KeyPair.new(vault, private: b_priv, public: b_pub, attrs: attrs)
    {:ok, kp3_shared} = Vault.ecdh(vault, kp3, kp4)
    {:ok, kp4_shared} = Vault.ecdh(vault, kp4, kp3)
    {:ok, ^shared} = Vault.export_secret(vault, kp3_shared)
    {:ok, ^shared} = Vault.export_secret(vault, kp4_shared)
  end
end
