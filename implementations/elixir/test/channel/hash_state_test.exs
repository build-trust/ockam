defmodule Ockam.Channel.HashState.Test do
  use ExUnit.Case, async: true

  alias Ockam.Vault
  alias Ockam.Vault.SecretAttributes
  alias Ockam.Channel.Protocol
  alias Ockam.Channel.HashState
  alias Ockam.Channel.CipherState

  setup do
    {:ok, vault} = Vault.new()
    {:ok, [vault: vault]}
  end

  test "Noise_XX_25519_AESGCM_SHA256", %{vault: vault} do
    {:ok, protocol} = Protocol.from_name("Noise_XX_25519_AESGCM_SHA256")

    sse0 = HashState.init(protocol, vault)
    ssd0 = HashState.init(protocol, vault)

    name = Protocol.name(protocol)
    pad_name = Vault.pad(name, Vault.hash_length(:sha256), 0)

    assert ^pad_name = HashState.h(sse0)
    assert {:ok, ^pad_name} = Vault.export_secret(vault, HashState.ck(sse0))
    refute CipherState.has_key(HashState.cipher_state(sse0))

    test_bin = Base.decode16!("6162636465666768696A6B6C6D6E6F707172737475767778797A")
    sse1 = HashState.mix_hash(sse0, vault, test_bin)
    ssd1 = HashState.mix_hash(ssd0, vault, test_bin)

    {:ok, exp_hash1} = Vault.hash(vault, :sha256, <<pad_name::binary, test_bin::binary>>)

    exp_hash2 = "\x14\xFB\xDE\x0EƳ۟^\xC1\xD6V\xC7*I\xB4\xFCLLW\x12\x87MAC_$\xF5q&\v\x96"

    assert ^exp_hash1 = HashState.h(sse1)
    assert ^exp_hash2 = HashState.h(ssd1)

    assert {:ok, sse2, test_bin} = HashState.encrypt_and_hash(sse1, vault, test_bin)
    assert {:ok, ssd2, test_bin} = HashState.decrypt_and_hash(ssd1, vault, test_bin)

    {:ok, test_ikm} = Vault.import_secret(vault, test_bin, SecretAttributes.buffer(:ephemeral))

    sse3 = HashState.mix_key(sse2, vault, test_ikm)
    ssd3 = HashState.mix_key(ssd2, vault, test_ikm)

    exp_encrypt =
      "j\xB6wN\xF8\xD2\xF6\xC9@3u\xF7\x8E⻾\xC8\xC4ǀ\x02\x87\x953\xE5|\xB5\x8B\v\v\xA8\x01y-$\xCF\tH6u\xA3\xD4"

    assert {:ok, sse4, encrypt} = HashState.encrypt_and_hash(sse3, vault, test_bin)
    assert ^exp_encrypt = encrypt
    assert {:ok, ssd4, decrypt} = HashState.decrypt_and_hash(ssd3, vault, exp_encrypt)
    assert ^test_bin = decrypt

    key1 = "Ka\xF7V\x82$e:\b\x91w\xF4}H\x11,̱\xBD\xA1D\x80\xC0\xE1\x1D\xA4\x0E[[V\xA0\xE5"
    key2 = "^\x1D\x14Kw\xBC\xAAm$C\xFF\b0\x88\xD3\xE8\x8D\xE2`\xBBj\xE5dD+nY!\"\x9E\xE7\xBC"

    {cse1, cse2} = HashState.split(sse4, vault)
    assert {:ok, ^key1} = Vault.export_secret(vault, CipherState.key(cse1))
    assert {:ok, ^key2} = Vault.export_secret(vault, CipherState.key(cse2))

    {csd1, csd2} = HashState.split(ssd4, vault)
    assert {:ok, ^key1} = Vault.export_secret(vault, CipherState.key(csd1))
    assert {:ok, ^key2} = Vault.export_secret(vault, CipherState.key(csd2))
  end
end
