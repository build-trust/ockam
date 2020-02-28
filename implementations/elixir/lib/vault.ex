defmodule Ockam.Vault do
  require Logger

  alias Ockam.Vault.KeyPair

  @type key_type :: :static | :ephemeral
  @type key_id :: binary()
  @type pubkey :: binary()
  @type privkey :: binary()
  @type keypair :: Ockam.Vault.KeyPair.t()
  @type salt :: binary()
  @type cipher :: :aes_256_gcm
  @type curve :: :x25519

  @hashes [:sha256, :sha512, :blake2s, :blake2b]
  @tag_size 16
  @max_nonce 0xFFFFFFFFFFFFFFFF

  def init_vault!(config) when is_list(config) do
    with {_, {:ok, curve}} <- {:curve, Keyword.fetch(config, :curve)},
         _ = :ets.new(__MODULE__.Keys, [:named_table, :set, :public]),
         _ = :ets.new(__MODULE__.Vaults, [:named_table, :set, :public]) do
      case Keyword.get(config, :keypairs, []) do
        [] ->
          :ok

        kps when is_list(kps) ->
          Logger.info("Registering configured static keys..")

          for {name, meta} <- kps do
            name = Atom.to_string(name)
            dir = Keyword.get(meta, :path, Application.app_dir(:ockam, :priv))
            Logger.debug("Registering key '#{name}' in directory '#{dir}'")
            pubkey_path = Path.join(dir, "#{name}.pub")
            privkey_path = Path.join(dir, "#{name}")
            pubkey = File.read!(pubkey_path)
            privkey = File.read!(privkey_path)
            true = :ets.insert_new(__MODULE__.Keys, {name, pubkey, privkey})
          end

          :ok
      end

      Logger.info("Vault initialized!")
    else
      error ->
        exit(error)
    end
  end

  defdelegate make_vault(curve), to: Ockam.Vault.NIF

  def random() do
    bytes = :crypto.strong_rand_bytes(8)
    {:ok, :crypto.bytes_to_integer(bytes)}
  end

  @spec key_gen_static(key_id) :: {:ok, {key_id, pubkey, privkey}}
  def key_gen_static(key_id) do
    case :ets.lookup(__MODULE__.Keys, key_id) do
      [] ->
        {pubkey, privkey} = :crypto.generate_key(:ecdh, :x25519)
        true = :ets.insert_new(__MODULE__.Keys, {key_id, pubkey, privkey})
        {:ok, {key_id, pubkey, privkey}}

      [{^key_id, _pubkey, _privkey} = found] ->
        {:ok, found}
    end
  end

  @spec key_gen_ephemeral() :: {:ok, {pubkey, privkey}}
  def key_gen_ephemeral() do
    {pubkey, privkey} = :crypto.generate_key(:ecdh, :x25519)
    {:ok, {pubkey, privkey}}
  end

  def get_public_key(:static, key_id) do
    case :ets.lookup(__MODULE__.Keys, key_id) do
      [] ->
        {:error, :not_found}

      [{^key_id, pubkey, _privkey}] ->
        {:ok, pubkey}
    end
  end

  def get_public_key(:ephemeral, _key_id) do
    {:error, {:invalid_key_type, :ephemeral}}
  end

  defdelegate write_public_key(vault, key_type, privkey), to: Ockam.Vault.NIF

  @doc """
  Hash some data using the given algorithm
  """
  def hash(algorithm, data)

  def hash(:sha256, data), do: sha256(data)
  def hash(:sha512, data), do: sha512(data)

  @doc "Hash some data with SHA-256"
  def sha256(data), do: :crypto.hash(:sha256, data)
  @doc "Hash some data with SHA-512"
  def sha512(data), do: :crypto.hash(:sha512, data)

  @doc """
  Perform a Diffie-Hellman calculation with the secret key from `us`
  and the public key from `them` with algorithm `curve`
  """
  @spec dh(curve(), keypair(), keypair()) :: binary()
  def dh(type, %KeyPair{} = us, %KeyPair{} = them) when type in [:x25519, :x448] do
    priv = KeyPair.private_key(us)
    pub = KeyPair.public_key(them)
    :crypto.compute_key(:ecdh, pub, priv, type)
  end

  def dh(type, _, _), do: :erlang.error({__MODULE__, {:unsupported_dh, type}})

  @rekey_size 32 * 8
  def rekey(:aes_256_gcm, key) do
    encrypt(:aes_256_gcm, key, @max_nonce, "", <<0::size(@rekey_size)>>)
  end

  @doc """
  Create a MAC using the HMAC algorithm
  """
  def hmac(hash, key, data)

  def hmac(hash, key, data)
      when hash in [:sha256, :sha512] and is_binary(key) and is_binary(data) do
    :crypto.mac(:hmac, hash, key, data)
  catch
    :error, {tag, {_file, _line}, description} ->
      :erlang.error({__MODULE__, {:hmac, tag, description}})
  end

  def hmac(hash, key, data)
      when hash in [:blake2b, :blake2s] and is_binary(key) and is_binary(data) do
    block_len = block_length(hash)
    block = hmac_format_key(hash, key, 0x36, block_len)
    hash_value = hash(hash, <<block::binary, data::binary>>)
    block = hmac_format_key(hash, key, 0x5C, block_len)
    hash(hash, <<block::binary, hash_value::binary>>)
  end

  defp hmac_format_key(hash, key, pad, block_len) do
    key =
      if byte_size(key) <= block_len do
        key
      else
        hash(hash, key)
      end

    key = pad(key, block_len, 0)

    <<padding::size(32)>> = <<pad::size(8), pad::size(8), pad::size(8), pad::size(8)>>

    for <<(<<word::size(32)>> <- key)>>, into: <<>> do
      <<:erlang.bxor(word, padding)::size(32)>>
    end
  end

  @doc """
  Perform HKDF on the given key and data
  """
  def hkdf(hash, key, data) when hash in @hashes and is_binary(key) and is_binary(data) do
    len = hash_length(hash)
    key = if key in [nil, ""], do: :binary.copy("", len), else: key
    data = if is_nil(data), do: "", else: data
    prk = hmac(hash, key, data)
    a = hmac(hash, prk, <<1::size(8)>>)
    b = hmac(hash, prk, a <> <<2::size(8)>>)
    c = hmac(hash, prk, b <> <<3::size(8)>>)
    [a, b, c]
  end

  @doc """
  Encrypt a message using the provided cipher
  """
  @spec encrypt(
          cipher,
          key :: binary(),
          nonce :: non_neg_integer(),
          aad :: binary(),
          plaintext :: binary()
        ) :: binary()
  def encrypt(:aes_256_gcm, key, nonce, aad, plaintext) do
    nonce = <<0::size(32), nonce::size(64)>>

    with {:ok, {ciphertext, tag}} <- aes_gcm_encrypt(plaintext, key, nonce, aad) do
      {:ok, <<ciphertext::binary, tag::binary>>}
    end
  end

  @doc """
  Decrypt a message using the provided cipher
  """
  @spec decrypt(
          cipher,
          key :: binary(),
          nonce :: non_neg_integer(),
          aad :: binary(),
          ciphertext :: binary()
        ) ::
          {:ok, binary()} | {:error, reason :: term}
  def decrypt(:aes_256_gcm, key, nonce, aad, ciphertext) do
    nonce = <<0::size(32), nonce::size(64)>>
    len = byte_size(ciphertext) - @tag_size
    <<ciphertext::size(len)-binary, tag::size(@tag_size)-binary>> = ciphertext
    aes_gcm_decrypt(ciphertext, key, nonce, aad, tag)
  end

  @doc """
  Encrypt a message using AES-256 GCM
  """
  @spec aes_gcm_encrypt(binary(), binary(), binary(), binary()) ::
          {:ok, {ciphertext :: binary, tag :: binary}} | {:error, term}
  def aes_gcm_encrypt(input, key, iv, aad) when is_binary(key) do
    {:ok, :crypto.crypto_one_time_aead(:aes_256_gcm, key, iv, input, aad, @tag_size, true)}
  catch
    :error, {tag, {_file, _line}, description} ->
      {:error, {tag, description}}
  end

  @doc """
  Decrypt a message encrypted with AES-256 GCM
  """
  def aes_gcm_decrypt(ciphertext, key, iv, aad, tag) when is_binary(key) do
    case :crypto.crypto_one_time_aead(:aes_256_gcm, key, iv, ciphertext, aad, tag, false) do
      :error ->
        {:error, {:decrypt, "decryption failed"}}

      plaintext ->
        {:ok, plaintext}
    end
  catch
    :error, {tag, {_file, _line}, description} ->
      {:error, {tag, description}}
  end

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
end
