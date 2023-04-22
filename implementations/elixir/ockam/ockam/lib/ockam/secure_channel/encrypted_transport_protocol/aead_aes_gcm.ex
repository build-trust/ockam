defmodule Ockam.SecureChannel.EncryptedTransportProtocol.AeadAesGcm do
  @moduledoc false

  alias Ockam.Vault
  alias __MODULE__
  @max_nonce :math.pow(2, 64) - 1

  defstruct [:vault, :k, :nonce]

  def new(vault, k, nonce) do
    %AeadAesGcm{vault: vault, k: k, nonce: nonce}
  end

  def encrypt(ad, plaintext, %AeadAesGcm{vault: vault, k: k, nonce: nonce} = state) do
    with {:ok, ciphertext} <- Vault.aead_aes_gcm_encrypt(vault, k, nonce, ad, plaintext),
         {:ok, next_nonce} <- increment_nonce(nonce) do
      {:ok, <<nonce::unsigned-big-integer-size(64), ciphertext::binary>>,
       %AeadAesGcm{state | nonce: next_nonce}}
    end
  end

  def decrypt(
        ad,
        <<nonce::unsigned-big-integer-size(64), ciphertext::binary>>,
        %AeadAesGcm{vault: vault, k: k, nonce: _expected_nonce} = state
      ) do
    with {:ok, plaintext} <- Vault.aead_aes_gcm_decrypt(vault, k, nonce, ad, ciphertext),
         {:ok, next_expected_nonce} <- increment_nonce(nonce) do
      {:ok, plaintext, %AeadAesGcm{state | nonce: next_expected_nonce}}
    end
  end

  defp increment_nonce(n) do
    case n + 1 do
      @max_nonce -> {:error, nil}
      valid_nonce -> {:ok, valid_nonce}
    end
  end
end
