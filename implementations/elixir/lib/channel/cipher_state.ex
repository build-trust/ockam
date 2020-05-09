defmodule Ockam.Channel.CipherState do
  alias Ockam.Vault
  alias Ockam.Vault.Secret

  @type cipher :: :aes_256_gcm | :chachapoly
  @type nonce :: non_neg_integer()
  @type key :: Secret.t() | nil

  @ciphers [:aes_256_gcm]

  defstruct k: nil, n: 0, cipher: :aes_256_gcm

  @type t :: %__MODULE__{
          k: key(),
          n: nonce(),
          cipher: :aes_256_gcm
        }

  def init(cipher), do: init(nil, cipher)

  def init(key, cipher) when cipher in @ciphers do
    do_init(key, cipher)
  end

  defp do_init(%Secret{} = key, cipher), do: %__MODULE__{k: key, n: 0, cipher: cipher}
  defp do_init(nil, cipher), do: %__MODULE__{k: nil, n: 0, cipher: cipher}

  def set_key(%__MODULE__{} = state, %Secret{} = key) do
    %__MODULE__{state | k: key, n: 0}
  end

  def has_key(%__MODULE__{k: k}), do: k != nil

  def set_nonce(%__MODULE__{} = state, nonce) do
    %__MODULE__{state | n: nonce}
  end

  def encrypt(%__MODULE__{k: nil} = state, _vault, _aad, plaintext) do
    {:ok, state, plaintext}
  end

  def encrypt(%__MODULE__{k: k, n: n, cipher: :aes_256_gcm} = state, vault, aad, plaintext) do
    with {:ok, ciphertext} <- Vault.encrypt(vault, k, n, aad, plaintext) do
      {:ok, %__MODULE__{state | n: n + 1}, ciphertext}
    end
  end

  def decrypt(%__MODULE__{k: nil} = state, _vault, _aad, ciphertext) do
    {:ok, state, ciphertext}
  end

  def decrypt(%__MODULE__{k: k, n: n, cipher: :aes_256_gcm} = state, vault, aad, ciphertext) do
    with {:ok, plaintext} <- Vault.decrypt(vault, k, n, aad, ciphertext) do
      {:ok, %__MODULE__{state | n: n + 1}, plaintext}
    end
  end

  def rekey(%__MODULE__{k: k, cipher: :aes_256_gcm} = cs, vault) do
    {:ok, k} = Vault.rekey(vault, k)
    %__MODULE__{cs | k: k}
  end

  def cipher(%__MODULE__{cipher: cipher}), do: cipher
  def key(%__MODULE__{k: k}), do: k
end
