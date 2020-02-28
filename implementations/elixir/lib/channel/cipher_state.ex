defmodule Ockam.Channel.CipherState do
  alias Ockam.Vault

  @type cipher :: :aes_256_gcm | :chachapoly
  @type nonce :: non_neg_integer()
  @type key :: binary() | nil

  @ciphers [:aes_256_gcm, :chachapoly]

  defstruct k: nil, n: 0, cipher: :aes_256_gcm

  @type t :: %__MODULE__{
          k: key(),
          n: nonce(),
          cipher: :aes_256_gcm
        }

  def init(cipher), do: init(nil, cipher)

  def init(key, cipher) when cipher in @ciphers and (is_nil(key) or is_binary(key)) do
    %__MODULE__{k: key, n: 0, cipher: cipher}
  end

  def set_key(%__MODULE__{} = state, key) do
    %__MODULE__{state | k: key, n: 0}
  end

  def has_key(%__MODULE__{k: k}), do: k != nil

  def set_nonce(%__MODULE__{} = state, nonce) do
    %__MODULE__{state | n: nonce}
  end

  def encrypt(%__MODULE__{k: nil} = state, _aad, plaintext) do
    {:ok, state, plaintext}
  end

  def encrypt(%__MODULE__{k: k, n: n, cipher: cipher} = state, aad, plaintext) do
    with {:ok, ciphertext} <- Vault.encrypt(cipher, k, n, aad, plaintext) do
      {:ok, %__MODULE__{state | n: n + 1}, ciphertext}
    end
  end

  def decrypt(%__MODULE__{k: nil} = state, _aad, ciphertext) do
    {:ok, state, ciphertext}
  end

  def decrypt(%__MODULE__{k: k, n: n, cipher: cipher} = state, aad, ciphertext) do
    with {:ok, plaintext} <- Vault.decrypt(cipher, k, n, aad, ciphertext) do
      {:ok, %__MODULE__{state | n: n + 1}, plaintext}
    end
  end

  def rekey(%__MODULE__{k: k, cipher: cipher}) do
    %__MODULE__{k: Vault.rekey(cipher, k)}
  end

  def cipher(%__MODULE__{cipher: cipher}), do: cipher
  def key(%__MODULE__{k: k}), do: k
end
