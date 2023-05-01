defmodule Ockam.SecureChannel.EncryptedTransportProtocol.AeadAesGcmTests do
  use ExUnit.Case, async: true

  alias Ockam.SecureChannel.EncryptedTransportProtocol.AeadAesGcm.Decryptor
  alias Ockam.SecureChannel.EncryptedTransportProtocol.AeadAesGcm.Encryptor
  alias Ockam.Vault
  alias Ockam.Vault.Software, as: SoftwareVault

  test "normal flow" do
    # We can't share the _same_ k between encryptor and decryptor on the same vault, as when the encryptor
    # rotate the key, it destroy the old k.  But that might still be used by the decryptor to decrypt yet-to-be
    # delivered packets.
    {:ok, encryptor_vault} = SoftwareVault.init()
    {:ok, decryptor_vault} = SoftwareVault.init()
    shared_k = :crypto.strong_rand_bytes(32)
    {:ok, ke} = Vault.secret_import(encryptor_vault, [type: :aes], shared_k)
    {:ok, kd} = Vault.secret_import(decryptor_vault, [type: :aes], shared_k)
    encryptor = Encryptor.new(encryptor_vault, ke, 0)
    decryptor = Decryptor.new(decryptor_vault, kd, 0)

    Enum.reduce(0..200, {encryptor, decryptor}, fn _i, {encryptor, decryptor} ->
      plain = :crypto.strong_rand_bytes(64)
      {:ok, ciphertext, encryptor} = Encryptor.encrypt(<<>>, plain, encryptor)
      {:ok, ^plain, decryptor} = Decryptor.decrypt(<<>>, ciphertext, decryptor)
      {encryptor, decryptor}
    end)
  end

  test "message lost" do
    {:ok, encryptor_vault} = SoftwareVault.init()
    {:ok, decryptor_vault} = SoftwareVault.init()
    shared_k = :crypto.strong_rand_bytes(32)
    {:ok, ke} = Vault.secret_import(encryptor_vault, [type: :aes], shared_k)
    {:ok, kd} = Vault.secret_import(decryptor_vault, [type: :aes], shared_k)
    encryptor = Encryptor.new(encryptor_vault, ke, 0, 32)
    decryptor = Decryptor.new(decryptor_vault, kd, 0, 32)

    Enum.reduce(0..200, {encryptor, decryptor}, fn i, {encryptor, decryptor} ->
      plain = :crypto.strong_rand_bytes(64)
      {:ok, ciphertext, encryptor} = Encryptor.encrypt(<<>>, plain, encryptor)

      if rem(i, 18) == 0 do
        {:ok, ^plain, decryptor} = Decryptor.decrypt(<<>>, ciphertext, decryptor)
        {encryptor, decryptor}
      else
        {encryptor, decryptor}
      end
    end)
  end

  test "out of order" do
    # We can't share the _same_ k between encryptor and decryptor on the same vault, as when the encryptor
    # rotate the key, it destroy the old k.  But that might still be used by the decryptor to decrypt yet-to-be
    # delivered packets.
    {:ok, encryptor_vault} = SoftwareVault.init()
    {:ok, decryptor_vault} = SoftwareVault.init()
    shared_k = :crypto.strong_rand_bytes(32)
    {:ok, ke} = Vault.secret_import(encryptor_vault, [type: :aes], shared_k)
    {:ok, kd} = Vault.secret_import(decryptor_vault, [type: :aes], shared_k)
    encryptor = Encryptor.new(encryptor_vault, ke, 0, 32)
    decryptor = Decryptor.new(decryptor_vault, kd, 0, 32)

    {msgs, encryptor} =
      Enum.reduce(0..1000, {[], encryptor}, fn i, {acc, encryptor} ->
        plain = :crypto.strong_rand_bytes(64)
        {:ok, ciphertext, encryptor} = Encryptor.encrypt(<<>>, plain, encryptor)
        {[{plain, ciphertext} | acc], encryptor}
      end)

    # msgs, elements up-to 10 position out of order
    msgs =
      msgs |> Enum.reverse() |> Enum.chunk_every(30) |> Enum.map(&Enum.shuffle/1) |> Enum.concat()

    # msgs can be decrypted
    decryptor =
      Enum.reduce(msgs, decryptor, fn {plain, ciphertext}, decryptor ->
        {:ok, ^plain, decryptor} = Decryptor.decrypt(<<>>, ciphertext, decryptor)
        decryptor
      end)

    # repeated nonces are detected
    Enum.each(msgs, fn {_plain, ciphertext} ->
      {:error, _} = Decryptor.decrypt(<<>>, ciphertext, decryptor)
    end)

    # good messages continue to be decrypted
    plain = :crypto.strong_rand_bytes(64)
    {:ok, ciphertext, _encryptor} = Encryptor.encrypt(<<>>, plain, encryptor)
    {:ok, ^plain, _decryptor} = Decryptor.decrypt(<<>>, ciphertext, decryptor)
  end
end
