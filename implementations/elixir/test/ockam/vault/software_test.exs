defmodule Ockam.Vault.Software.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Vault.Software

  describe "Ockam.Vault.Software.sha256/2" do
    test "can run natively implemented functions" do
      handle = Ockam.Vault.Software.default_init
      assert Ockam.Vault.Software.sha256(handle, "test") == <<0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0>>
    end
  end
end
