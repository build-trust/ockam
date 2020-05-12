defmodule Ockam.Channel.HashState do
  @moduledoc "Noise symmetric (hash) state"

  alias Ockam.Vault
  alias Ockam.Vault.Secret
  alias Ockam.Vault.SecretAttributes
  alias Ockam.Channel.Protocol
  alias Ockam.Channel.CipherState

  @type hash :: :sha256

  defstruct cs: nil, ck: "", h: "", hash: :sha256

  @type t :: %__MODULE__{
          cs: CipherState.t(),
          ck: binary(),
          h: binary(),
          hash: hash()
        }

  @spec init(Protocol.t(), Vault.t()) :: t()
  def init(%Protocol{} = protocol, %Vault{} = vault) do
    hash = Protocol.hash(protocol)
    cipher = Protocol.cipher(protocol)
    name = Protocol.name(protocol)
    hash_len = Vault.hash_length(hash)

    h =
      if byte_size(name) > hash_len do
        Vault.hash(vault, hash, name)
      else
        Vault.pad(name, hash_len, 0x00)
      end

    {:ok, ck} = Vault.import_secret(vault, h, SecretAttributes.buffer(:ephemeral))

    %__MODULE__{
      cs: CipherState.init(cipher),
      ck: ck,
      h: h,
      hash: hash
    }
  end

  def mix_key(%__MODULE__{ck: ck, cs: cs} = state, vault, %Secret{} = ikm) do
    {:ok, [ck, temp_k]} = Vault.hkdf(vault, ck, ikm, 2)
    {:ok, new_temp_k} = Vault.set_secret_type(vault, temp_k, :aes256)
    %__MODULE__{state | ck: ck, cs: CipherState.set_key(cs, new_temp_k)}
  end

  def mix_hash(%__MODULE__{} = state, vault, nil), do: mix_hash(state, vault, "")

  def mix_hash(%__MODULE__{hash: hash, h: h} = state, vault, data) do
    {:ok, h} = Vault.hash(vault, hash, <<h::binary, data::binary>>)
    %__MODULE__{state | h: h}
  end

  def mix_key_and_hash(%__MODULE__{ck: ck, cs: cs} = state, vault, %Secret{} = ikm) do
    {:ok, [ck, temp_h, temp_k]} = Vault.hkdf(vault, ck, ikm, 3)
    {:ok, new_temp_k} = Vault.set_secret_type(vault, temp_k, :aes256)
    cs = CipherState.set_key(cs, new_temp_k)
    mix_hash(%__MODULE__{state | ck: ck, cs: cs}, vault, temp_h)
  end

  def encrypt_and_hash(%__MODULE__{cs: cs, h: h} = state, vault, plaintext) do
    {:ok, cs, ciphertext} = CipherState.encrypt(cs, vault, h, plaintext)
    {:ok, mix_hash(%__MODULE__{state | cs: cs}, vault, ciphertext), ciphertext}
  end

  def decrypt_and_hash(%__MODULE__{cs: cs, h: h} = state, vault, ciphertext) do
    with {:ok, cs, plaintext} <- CipherState.decrypt(cs, vault, h, ciphertext) do
      {:ok, mix_hash(%__MODULE__{state | cs: cs}, vault, ciphertext), plaintext}
    end
  end

  def split(%__MODULE__{ck: ck, cs: cs}, vault) do
    {:ok,
     [
       temp_k1,
       temp_k2
     ]} = Vault.hkdf(vault, ck, nil, 2)

    {:ok, new_temp_k1} = Vault.set_secret_type(vault, temp_k1, :aes256)
    {:ok, new_temp_k2} = Vault.set_secret_type(vault, temp_k2, :aes256)
    {CipherState.set_key(cs, new_temp_k1), CipherState.set_key(cs, new_temp_k2)}
  end

  def cipher_state(%__MODULE__{cs: cs}), do: cs
  def ck(%__MODULE__{ck: ck}), do: ck
  def h(%__MODULE__{h: h}), do: h
  def hash(%__MODULE__{hash: hash}), do: hash
end
