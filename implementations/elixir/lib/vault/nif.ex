defmodule Ockam.Vault.NIF do
  use Rustler,
    otp_app: :ockam,
    crate: :ockam_nif

  alias Ockam.Vault.SecretAttributes

  @type error :: atom()
  @type vault_result(ty) :: {:ok, ty} | {:error, error()}
  @type may_fail :: :ok | :error
  @opaque vault :: reference()
  @opaque secret :: reference()

  @type secret_persistence :: SecretAttributes.secret_persistence()
  @type secret_type :: SecretAttributes.secret_type()
  @type secret_purpose :: SecretAttributes.secret_purpose()

  @type salt :: binary()
  @type key :: binary()
  @type pubkey :: key()
  @type privkey :: key()
  @type iv :: binary()
  @type tag :: binary()
  @type bytes :: binary()
  @type option(t) :: nil | t

  @spec make_vault() :: vault_result(vault)
  def make_vault() do
    exit(:nif_not_loaded)
  end

  @spec random(vault()) :: non_neg_integer()
  def random(_vault) do
    exit(:nif_not_loaded)
  end

  @spec sha256(vault(), binary()) :: vault_result(binary())
  def sha256(_vault, _data) do
    exit(:nif_not_loaded)
  end

  @spec generate_secret(vault(), SecretAttributes.t()) :: vault_result(secret())
  def generate_secret(_vault, _attrs) do
    exit(:nif_not_loaded)
  end

  @spec import_secret(vault(), binary(), SecretAttributes.t()) :: vault_result(secret())
  def import_secret(_vault, _data, _attrs) do
    exit(:nif_not_loaded)
  end

  @spec export_secret(vault(), secret()) :: vault_result(binary())
  def export_secret(_vault, _secret) do
    exit(:nif_not_loaded)
  end

  @spec get_secret_attributes(vault(), secret()) :: vault_result(SecretAttributes.t())
  def get_secret_attributes(_vault, _secret) do
    exit(:nif_not_loaded)
  end

  @spec set_secret_type(vault(), secret(), secret_type()) :: may_fail
  def set_secret_type(_vault, _secret, _secret_type) do
    exit(:nif_not_loaded)
  end

  @spec get_public_key(vault(), secret()) :: vault_result(binary())
  def get_public_key(_vault, _secret) do
    exit(:nif_not_loaded)
  end

  @spec ecdh(vault(), secret(), binary()) :: vault_result(secret())
  def ecdh(_vault, _private_key, _peer_pubkey) do
    exit(:nif_not_loaded)
  end

  @spec hkdf_sha256(
          vault(),
          salt :: secret(),
          input_key_material :: secret(),
          num_derived_outputs :: non_neg_integer()
        ) ::
          vault_result([secret()])
  def hkdf_sha256(_vault, _salt, _ikm, _num_derived_outputs) do
    exit(:nif_not_loaded)
  end

  @spec aead_aes_gcm_encrypt(
          vault(),
          key :: secret(),
          nonce :: non_neg_integer(),
          additional_data :: option(binary()),
          plaintext :: binary()
        ) ::
          vault_result(binary())
  def aead_aes_gcm_encrypt(_vault, _key, _nonce, _additional_data, _plaintext) do
    exit(:nif_not_loaded)
  end

  @spec aead_aes_gcm_decrypt(
          vault(),
          key :: secret(),
          nonce :: non_neg_integer(),
          additional_data :: option(binary()),
          ciphertext_and_tag :: binary()
        ) ::
          vault_result(binary())
  def aead_aes_gcm_decrypt(_vault, _key, _nonce, _additional_data, _ciphertext_and_tag) do
    exit(:nif_not_loaded)
  end
end
