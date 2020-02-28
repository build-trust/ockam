defmodule Ockam.Channel.HashState do
  @moduledoc "Noise symmetric (hash) state"

  alias Ockam.Vault
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

  @spec init(Protocol.t()) :: t()
  def init(%Protocol{} = protocol) do
    hash = Protocol.hash(protocol)
    cipher = Protocol.cipher(protocol)
    name = Protocol.name(protocol)
    hash_len = Vault.hash_length(hash)

    h =
      if byte_size(name) > hash_len do
        Vault.hash(hash, name)
      else
        Vault.pad(name, hash_len, 0x00)
      end

    %__MODULE__{
      cs: CipherState.init(cipher),
      ck: h,
      h: h,
      hash: hash
    }
  end

  def mix_key(%__MODULE__{hash: hash, ck: ck, cs: cs} = state, ikm) do
    [ck, <<temp_k::size(32)-binary, _::binary>> | _] = Vault.hkdf(hash, ck, ikm)
    %__MODULE__{state | ck: ck, cs: CipherState.set_key(cs, temp_k)}
  end

  def mix_hash(%__MODULE__{} = state, nil), do: mix_hash(state, "")

  def mix_hash(%__MODULE__{hash: hash, h: h} = state, data) do
    h = Vault.hash(hash, <<h::binary, data::binary>>)
    %__MODULE__{state | h: h}
  end

  def mix_key_and_hash(%__MODULE__{hash: hash, ck: ck, cs: cs} = state, ikm) do
    [ck, temp_h, <<temp_k::size(32)-binary, _::binary>>] = Vault.hkdf(hash, ck, ikm)
    cs = CipherState.set_key(cs, temp_k)
    mix_hash(%__MODULE__{state | ck: ck, cs: cs}, temp_h)
  end

  def encrypt_and_hash(%__MODULE__{cs: cs, h: h} = state, plaintext) do
    {:ok, cs, ciphertext} = CipherState.encrypt(cs, h, plaintext)
    {:ok, mix_hash(%__MODULE__{state | cs: cs}, ciphertext), ciphertext}
  end

  def decrypt_and_hash(%__MODULE__{cs: cs, h: h} = state, ciphertext) do
    with {:ok, cs, plaintext} <- CipherState.decrypt(cs, h, ciphertext) do
      {:ok, mix_hash(%__MODULE__{state | cs: cs}, ciphertext), plaintext}
    end
  end

  def split(%__MODULE__{hash: hash, ck: ck, cs: cs}) do
    [
      <<temp_k1::size(32)-binary, _::binary>>,
      <<temp_k2::size(32)-binary, _::binary>>,
      _
    ] = Vault.hkdf(hash, ck, "")

    {CipherState.set_key(cs, temp_k1), CipherState.set_key(cs, temp_k2)}
  end

  def cipher_state(%__MODULE__{cs: cs}), do: cs
  def ck(%__MODULE__{ck: ck}), do: ck
  def h(%__MODULE__{h: h}), do: h
  def hash(%__MODULE__{hash: hash}), do: hash
end
