defmodule Ockam.Vault.Tests do
  use ExUnit.Case, async: true

  alias Ockam.Vault
  alias Ockam.Vault.KeyPair
  alias Ockam.Test.Fixtures

  test "curve25519" do
    kp1 = KeyPair.new(:x25519)
    kp2 = KeyPair.new(:x25519)

    a = Vault.dh(:x25519, kp1, kp2)
    b = Vault.dh(:x25519, kp2, kp1)
    assert ^a = b

    %{a_pub: a_pub, a_priv: a_priv, b_pub: b_pub, b_priv: b_priv, shared: shared} =
      Fixtures.curve25519_data()

    kp3 = KeyPair.new(:x25519, private: a_priv, public: a_pub)
    kp4 = KeyPair.new(:x25519, private: b_priv, public: b_pub)
    assert ^shared = Vault.dh(:x25519, kp3, kp4)
    assert ^shared = Vault.dh(:x25519, kp4, kp3)
  end
end
