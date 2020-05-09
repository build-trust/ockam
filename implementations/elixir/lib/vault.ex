defmodule Ockam.Vault do
  require Logger

  alias __MODULE__.NIF
  alias Ockam.Vault.KeyPair
  alias Ockam.Vault.Secret
  alias Ockam.Vault.SecretAttributes

  defstruct [:context]

  @opaque t :: %__MODULE__{}

  @doc """
  Create a new instance of a Vault
  """
  @spec new() :: {:ok, t} | {:error, term}
  def new() do
    with {:ok, vault} = NIF.make_vault() do
      {:ok, %__MODULE__{context: vault}}
    end
  end

  @doc """
  Generate a new unique random 32-bit integer value
  """
  @spec random(t) :: non_neg_integer()
  def random(%__MODULE__{context: context}) do
    NIF.random(context)
  end

  @doc """
  Hash some data using the given algorithm
  """
  def hash(t, algorithm, data)

  def hash(vault, :sha256, data), do: sha256(vault, data)
  def hash(_vault, :sha512, data), do: sha512(data)

  @doc "Hash some data with SHA-256"
  def sha256(%__MODULE__{context: context}, data) when is_binary(data),
    do: NIF.sha256(context, data)

  @doc "Hash some data with SHA-512"
  def sha512(data), do: :crypto.hash(:sha512, data)

  @spec generate_secret(t, SecretAttributes.t()) :: {:ok, Secret.t()} | {:error, term}
  def generate_secret(%__MODULE__{context: context}, %SecretAttributes{} = attrs) do
    with {:ok, secret} <- NIF.generate_secret(context, attrs) do
      {:ok, Secret.new(secret, attrs)}
    end
  end

  @spec import_secret(t, binary(), SecretAttributes.t()) :: {:ok, Secret.t()} | {:error, term}
  def import_secret(%__MODULE__{context: context}, data, %SecretAttributes{} = attrs) do
    with {:ok, secret} <- NIF.import_secret(context, data, attrs) do
      {:ok, Secret.new(secret, attrs)}
    end
  end

  @spec export_secret(t, Secret.t()) :: {:ok, binary()} | {:error, term}
  def export_secret(%__MODULE__{context: context}, %Secret{secret: secret}) do
    NIF.export_secret(context, secret)
  end

  @spec get_secret_attributes(t, Secret.t()) :: {:ok, SecretAttributes.t()} | {:error, term}
  def get_secret_attributes(%__MODULE__{context: context}, %Secret{secret: secret}) do
    NIF.get_secret_attributes(context, secret)
  end

  @spec set_secret_type(t, Secret.t(), SecretAttributes.secret_type()) :: :ok | {:error, term}
  def set_secret_type(
        %__MODULE__{context: context},
        %Secret{secret: secret, attrs: attrs} = s,
        ty
      ) do
    with {:ok, new_attrs} <- SecretAttributes.set_type(attrs, ty),
         :ok <- NIF.set_secret_type(context, secret, ty) do
      {:ok, %Secret{s | attrs: new_attrs}}
    end
  end

  @spec get_public_key(t, Secret.t()) :: {:ok, binary()} | {:error, term}
  def get_public_key(%__MODULE__{context: context}, %Secret{secret: secret}) do
    NIF.get_public_key(context, secret)
  end

  @doc """
  Perform a Diffie-Hellman calculation with the secret key from `us`
  and the public key from `them` with algorithm `curve`
  """
  @spec ecdh(t, KeyPair.t(), KeyPair.t()) :: {:ok, Secret.t()} | {:error, term}
  def ecdh(%__MODULE__{context: context}, %KeyPair{} = us, %KeyPair{} = them) do
    do_ecdh(context, KeyPair.private_key(us), KeyPair.public_key(them))
  end

  defp do_ecdh(vault, %Secret{secret: privkey}, pubkey) when is_binary(pubkey) do
    with {:ok, secret} <- NIF.ecdh(vault, privkey, pubkey),
         {:ok, attrs} <- NIF.get_secret_attributes(vault, secret) do
      {:ok, Secret.new(secret, attrs)}
    end
  end

  @doc """
  Perform HKDF on the given key and data
  """
  @spec hkdf(t, Secret.t(), Secret.t(), num_outputs :: pos_integer()) :: {:ok, [Secret.t()]}
  def hkdf(
        %__MODULE__{context: context},
        %Secret{secret: salt},
        %Secret{secret: ikm},
        num_outputs
      )
      when is_integer(num_outputs) and num_outputs > 0 do
    with {:ok, result} <- NIF.hkdf_sha256(context, salt, ikm, num_outputs) do
      secrets =
        for secret <- result do
          case NIF.get_secret_attributes(context, secret) do
            {:ok, attrs} ->
              Secret.new(secret, attrs)

            {:error, reason} ->
              throw(reason)
          end
        end

      {:ok, secrets}
    end
  catch
    :throw, reason ->
      {:error, reason}
  end

  @doc """
  Encrypt a message using the provided cipher
  """
  @spec encrypt(
          t,
          Secret.t(),
          nonce :: non_neg_integer(),
          aad :: binary(),
          plaintext :: binary()
        ) :: {:ok, binary()} | {:error, term}
  def encrypt(%__MODULE__{context: context}, %Secret{secret: key}, nonce, aad, plaintext)
      when is_integer(nonce) do
    NIF.aead_aes_gcm_encrypt(context, key, nonce, aad, plaintext)
  end

  @doc """
  Decrypt a message using the provided cipher
  """
  @spec decrypt(
          t,
          Secret.t(),
          nonce :: non_neg_integer(),
          aad :: binary(),
          ciphertext_and_tag :: binary()
        ) :: {:ok, binary()} | {:error, reason :: term}
  def decrypt(%__MODULE__{context: context}, %Secret{secret: key}, nonce, aad, ciphertext_and_tag)
      when is_integer(nonce) do
    NIF.aead_aes_gcm_decrypt(context, key, nonce, aad, ciphertext_and_tag)
  end

  @max_nonce 0xFFFFFFFFFFFFFFFF
  @rekey_size 32 * 8
  def rekey(%__MODULE__{} = vault, key) do
    encrypt(vault, key, @max_nonce, "", <<0::size(@rekey_size)>>)
  end

  @doc "Get the length in bytes of the given hash algorithm output"
  def hash_length(:sha256), do: 32
  def hash_length(:sha512), do: 64
  def hash_length(:blake2s), do: 32
  def hash_length(:blake2b), do: 64

  @doc "Get the block size in bytes of the given hash algorithm"
  def block_length(:sha256), do: 64
  def block_length(:sha512), do: 128
  def block_length(:blake2s), do: 64
  def block_length(:blake2b), do: 128

  @doc "Get the key size in bytes of the given Diffie-Hellman algorithm"
  def dh_length(:x25519), do: 32
  def dh_length(:x448), do: 56

  @doc "Pad data to at least `min_size`, using `pad_byte` to fill padding bytes"
  def pad(data, min_size, pad_byte)
      when is_binary(data) and min_size >= 0 and is_integer(pad_byte) and pad_byte <= 255 do
    case byte_size(data) do
      n when n >= min_size ->
        data

      n ->
        padding = for _ <- 1..(min_size - n), do: <<pad_byte::size(8)>>, into: <<>>
        <<data::binary, padding::binary>>
    end
  end
end
