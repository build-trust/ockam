defmodule Ockam.SecureChannel.EncryptedTransportProtocol.AeadAesGcmTests do
  use ExUnit.Case, async: true

  alias Ockam.SecureChannel.EncryptedTransportProtocol.AeadAesGcm, as: EncryptedTransport
  alias Ockam.Vault
  alias Ockam.Vault.Software, as: SoftwareVault

  # Test aren't very intersting for now,  but will come handy once we add rekey and out-of-order window
  # handling.  Easy to simulate lost of packets,  replays, etc.
  test "normal flow" do
    {:ok, vault} = SoftwareVault.init()
    {:ok, k} = Vault.secret_import(vault, [type: :aes], :crypto.strong_rand_bytes(32))
    encryptor = EncryptedTransport.new(vault, k, 0)
    decryptor = EncryptedTransport.new(vault, k, 0)

    Enum.reduce(0..100, {encryptor, decryptor}, fn _i, {encryptor, decryptor} ->
      plain = :crypto.strong_rand_bytes(64)
      {:ok, ciphertext, encryptor} = EncryptedTransport.encrypt(<<>>, plain, encryptor)
      {:ok, ^plain, decryptor} = EncryptedTransport.decrypt(<<>>, ciphertext, decryptor)
      {encryptor, decryptor}
    end)
  end
end
