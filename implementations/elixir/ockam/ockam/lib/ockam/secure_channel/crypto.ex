defmodule Ockam.SecureChannel.Crypto do
  @moduledoc """
    Crypto functions used in secure channel
  """

  def sha256(value) do
    :crypto.hash(:sha256, value)
  end

  def generate_dh_keypair() do
    {pub_key, secret_key} = :crypto.generate_key(:eddh, :x25519)
    {:ok, %{private: secret_key, public: pub_key}}
  end

  def dh(peer_public, private) do
    {:ok, :crypto.compute_key(:ecdh, peer_public, private, :x25519)}
  end

  def aead_aes_gcm_encrypt(k, n, h, plaintext) do
    with {a, b} <- :crypto.crypto_one_time_aead(:aes_256_gcm, k, <<n::96>>, plaintext, h, true) do
      {:ok, <<a::binary, b::binary>>}
    end
  end

  def aead_aes_gcm_decrypt(k, n, h, ciphertext_and_tag) do
    size = byte_size(ciphertext_and_tag) - 16
    <<ciphertext::binary-size(size), tag::binary-size(16)>> = ciphertext_and_tag

    case :crypto.crypto_one_time_aead(:aes_256_gcm, k, <<n::96>>, ciphertext, h, tag, false) do
      :error -> {:error, :aead_aes_gcm_decrypt_error}
      plaintext -> {:ok, plaintext}
    end
  end

  def hkdf(salt), do: hkdf(salt, <<>>)

  def hkdf(salt, ikm) do
    <<k1::binary-size(32), k2::binary-size(32)>> = :hkdf.derive(:sha256, ikm, "", salt, 64)
    {k1, k2}
  end
end
