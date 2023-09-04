defmodule Ockam.SecureChannel.EncryptedTransportProtocol.AeadAesGcmTests do
  use ExUnit.Case, async: true

  alias Ockam.SecureChannel.EncryptedTransportProtocol.AeadAesGcm.Decryptor
  alias Ockam.SecureChannel.EncryptedTransportProtocol.AeadAesGcm.Encryptor

  test "normal flow" do
    shared_k = :crypto.strong_rand_bytes(32)
    encryptor = Encryptor.new(shared_k, 0)
    decryptor = Decryptor.new(shared_k, 0)

    Enum.reduce(0..200, {encryptor, decryptor}, fn _i, {encryptor, decryptor} ->
      plain = :crypto.strong_rand_bytes(64)
      {:ok, ciphertext, encryptor} = Encryptor.encrypt(<<>>, plain, encryptor)
      {:ok, ^plain, decryptor} = Decryptor.decrypt(<<>>, ciphertext, decryptor)
      {encryptor, decryptor}
    end)
  end

  test "message lost" do
    shared_k = :crypto.strong_rand_bytes(32)
    encryptor = Encryptor.new(shared_k, 0, 32)
    decryptor = Decryptor.new(shared_k, 0, 32)

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
    shared_k = :crypto.strong_rand_bytes(32)
    encryptor = Encryptor.new(shared_k, 0, 32)
    decryptor = Decryptor.new(shared_k, 0, 32)

    {msgs, encryptor} =
      Enum.reduce(0..1000, {[], encryptor}, fn _i, {acc, encryptor} ->
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

  test "out of order, exact sliding window" do
    # Test values taken from nonce_tracker.rs test case
    shared_k = :crypto.strong_rand_bytes(32)
    key_renewal_interval = 32
    encryptor = Encryptor.new(shared_k, 0, key_renewal_interval)
    decryptor = Decryptor.new(shared_k, 0, key_renewal_interval)

    {msgs, _encryptor} =
      Enum.reduce(0..(key_renewal_interval * 5), {[], encryptor}, fn _i, {acc, encryptor} ->
        plain = :crypto.strong_rand_bytes(64)
        {:ok, ciphertext, encryptor} = Encryptor.encrypt(<<>>, plain, encryptor)
        {[{plain, ciphertext} | acc], encryptor}
      end)

    msgs = msgs |> Enum.reverse()

    {plaintext, ciphertext} = Enum.at(msgs, 0)
    {:ok, ^plaintext, decryptor} = Decryptor.decrypt(<<>>, ciphertext, decryptor)

    {plaintext, ciphertext} = Enum.at(msgs, 1)
    {:ok, ^plaintext, decryptor} = Decryptor.decrypt(<<>>, ciphertext, decryptor)

    {_, ciphertext} = Enum.at(msgs, 0)
    {:error, _} = Decryptor.decrypt(<<>>, ciphertext, decryptor)

    {_plaintext, ciphertext} = Enum.at(msgs, key_renewal_interval + 2)
    {:error, _} = Decryptor.decrypt(<<>>, ciphertext, decryptor)

    {plaintext, ciphertext} = Enum.at(msgs, key_renewal_interval + 1)
    {:ok, ^plaintext, decryptor} = Decryptor.decrypt(<<>>, ciphertext, decryptor)

    {_plaintext, ciphertext} = Enum.at(msgs, 1)
    {:error, _} = Decryptor.decrypt(<<>>, ciphertext, decryptor)

    {plaintext, ciphertext} = Enum.at(msgs, key_renewal_interval + 2)
    {:ok, ^plaintext, decryptor} = Decryptor.decrypt(<<>>, ciphertext, decryptor)
    {plaintext, ciphertext} = Enum.at(msgs, key_renewal_interval + 3)
    {:ok, ^plaintext, decryptor} = Decryptor.decrypt(<<>>, ciphertext, decryptor)

    {_plaintext, ciphertext} = Enum.at(msgs, key_renewal_interval + 1)
    {:error, _} = Decryptor.decrypt(<<>>, ciphertext, decryptor)
    {_plaintext, ciphertext} = Enum.at(msgs, key_renewal_interval + 2)
    {:error, _} = Decryptor.decrypt(<<>>, ciphertext, decryptor)

    {plaintext, ciphertext} = Enum.at(msgs, 2 * key_renewal_interval)
    {:ok, ^plaintext, decryptor} = Decryptor.decrypt(<<>>, ciphertext, decryptor)

    {_plaintext, ciphertext} = Enum.at(msgs, key_renewal_interval - 1)
    {:error, _} = Decryptor.decrypt(<<>>, ciphertext, decryptor)

    {plaintext, ciphertext} = Enum.at(msgs, 3 * key_renewal_interval)
    {:ok, ^plaintext, decryptor} = Decryptor.decrypt(<<>>, ciphertext, decryptor)
    {plaintext, ciphertext} = Enum.at(msgs, 4 * key_renewal_interval)
    {:ok, ^plaintext, decryptor} = Decryptor.decrypt(<<>>, ciphertext, decryptor)

    decryptor =
      Enum.reduce((3 * key_renewal_interval + 1)..(4 * key_renewal_interval - 1), decryptor, fn i,
                                                                                                decryptor ->
        {plaintext, ciphertext} = Enum.at(msgs, i)
        {:ok, ^plaintext, decryptor} = Decryptor.decrypt(<<>>, ciphertext, decryptor)
        decryptor
      end)

    Enum.reduce((4 * key_renewal_interval + 1)..(5 * key_renewal_interval), decryptor, fn i,
                                                                                          decryptor ->
      {plaintext, ciphertext} = Enum.at(msgs, i)
      {:ok, ^plaintext, decryptor} = Decryptor.decrypt(<<>>, ciphertext, decryptor)
      decryptor
    end)
  end
end
