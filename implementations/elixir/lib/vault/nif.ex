defmodule Ockam.Vault.NIF do
  use Rustler,
    otp_app: :ockam,
    crate: :ockam_nif

  @type curve :: :p256 | :curve25519
  @type key_type :: :static | :ephemeral
  @type salt :: binary()
  @type key :: binary()
  @type pubkey :: key()
  @type privkey :: key()
  @type iv :: binary()
  @type tag :: binary()
  @type bytes :: binary()
  @type option(t) :: nil | t
  @opaque vault :: any()

  @spec make_vault(curve()) :: {:ok, vault} | {:error, atom()}
  def make_vault(_curve) do
    exit(:nif_not_loaded)
  end

  @spec random(vault()) :: non_neg_integer()
  def random(_vault) do
    exit(:nif_not_loaded)
  end

  @spec key_gen(vault(), key_type()) :: :ok | {:error, atom()}
  def key_gen(_vault, _key_type) do
    exit(:nif_not_loaded)
  end

  @spec get_public_key(vault(), key_type()) :: {:ok, binary()} | {:error, atom()}
  def get_public_key(_vault, _key_type) do
    exit(:nif_not_loaded)
  end

  @spec write_public_key(vault(), key_type(), binary()) :: :ok | {:error, atom()}
  def write_public_key(_vault, _key_type, _privkey) do
    exit(:nif_not_loaded)
  end

  @spec ecdh(vault(), key_type(), binary()) :: {:ok, binary()} | {:error, atom()}
  def ecdh(_vault, _key_type, _pubkey) do
    exit(:nif_not_loaded)
  end

  @spec hkdf(vault(), salt(), pubkey(), info :: option(binary())) ::
          {:ok, binary()} | {:error, atom()}
  def hkdf(_vault, _salt, _key, _info) do
    exit(:nif_not_loaded)
  end

  @spec aes_gcm_encrypt(vault(), bytes(), key(), iv(), data :: option(binary()), tag()) ::
          {:ok, binary()} | {:error, atom()}
  def aes_gcm_encrypt(_vault, _input, _key, _iv, _additional_data, _tag) do
    exit(:nif_not_loaded)
  end

  @spec aes_gcm_decrypt(vault(), bytes(), key(), iv(), data :: option(binary()), tag()) ::
          {:ok, binary()} | {:error, atom()}
  def aes_gcm_decrypt(_vault, _input, _key, _iv, _additional_data, _tag) do
    exit(:nif_not_loaded)
  end
end
