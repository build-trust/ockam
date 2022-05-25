defmodule Ockam.Vault do
  @moduledoc false

  ## NIF functions always infer as any()
  ## The types are useful for readability
  @dialyzer [:no_contracts]

  @default_secret_attributes [type: :curve25519, persistence: :ephemeral, length: 32]

  @doc """
    Computes a SHA-256 hash based on input data.
  """
  @spec sha256(Ockam.Vault, binary | String.t()) :: {:ok, binary} | :error
  def sha256(%vault_module{id: vault_id}, input) do
    vault_module.sha256(vault_id, input)
  end

  @doc """
    Fills output_buffer with randomly generate bytes.
  """
  @spec random_bytes(Ockam.Vault, binary) :: :error
  def random_bytes(%vault_module{id: vault_id}, output_buffer) do
    vault_module.random_bytes(vault_id, output_buffer)
  end

  @doc """
    Generates an ockam secret. Attributes struct must specify the
    configuration for the type of secret to generate.
  """
  @spec secret_generate(Ockam.Vault, keyword()) :: {:ok, reference()} | :error
  def secret_generate(%vault_module{id: vault_id}, attributes) when is_list(attributes) do
    attributes = @default_secret_attributes |> Keyword.merge(attributes) |> Map.new()
    vault_module.secret_generate(vault_id, attributes)
  end

  @doc """
    Imports the specified data into the supplied ockam vault secret.
  """
  @spec secret_import(Ockam.Vault, keyword(), binary) :: {:ok, reference()} | :error
  def secret_import(%vault_module{id: vault_id}, attributes, input) when is_list(attributes) do
    attributes = @default_secret_attributes |> Keyword.merge(attributes) |> Map.new()
    vault_module.secret_import(vault_id, attributes, input)
  end

  @doc """
    Exports data from an ockam vault secret into the supplied output buffer.
  """
  @spec secret_export(Ockam.Vault, reference()) :: {:ok, binary} | :error
  def secret_export(%vault_module{id: vault_id}, secret_handle) do
    vault_module.secret_export(vault_id, secret_handle)
  end

  @doc """
    Retrieves the public key from an ockam vault secret.
  """
  @spec secret_publickey_get(Ockam.Vault, reference()) :: {:ok, reference()} | :error
  def secret_publickey_get(%vault_module{id: vault_id}, secret_handle) do
    vault_module.secret_publickey_get(vault_id, secret_handle)
  end

  @doc """
    Retrieves the attributes for a specified secret
  """
  @spec secret_attributes_get(Ockam.Vault, reference()) :: {:ok, keyword()} | :error
  def secret_attributes_get(%vault_module{id: vault_id}, secret_handle) do
    with {:ok, attributes} <- vault_module.secret_attributes_get(vault_id, secret_handle) do
      {:ok, Map.to_list(attributes)}
    end
  end

  @doc """
    Deletes an ockam vault secret.
  """
  @spec secret_destroy(Ockam.Vault, reference()) :: :ok | :error
  def secret_destroy(%vault_module{id: vault_id}, secret_handle) do
    vault_module.secret_destroy(vault_id, secret_handle)
  end

  @doc """
    Performs an ECDH operation on the supplied ockam vault secret and peer_publickey.
    The result is another ockam vault secret of type unknown.
  """
  @spec ecdh(Ockam.Vault, reference(), binary) :: {:ok, reference()} | :error
  def ecdh(%vault_module{id: vault_id}, secret_handle, peer_public_key) do
    vault_module.ecdh(vault_id, secret_handle, peer_public_key)
  end

  @doc """
    Performs an HMAC-SHA256 based key derivation function on the supplied salt and input
    key material.
    Returns handle to derived_output.
  """
  @spec hkdf_sha256(Ockam.Vault, reference(), reference(), non_neg_integer()) ::
          {:ok, reference()} | :error
  def hkdf_sha256(%vault_module{id: vault_id}, salt_handle, ikm_handle, derived_outputs_count) do
    vault_module.hkdf_sha256(vault_id, salt_handle, ikm_handle, derived_outputs_count)
  end

  @doc """
    Performs an HMAC-SHA256 based key derivation function on the supplied salt and input key
    material.
    Returns handle to derived_output.
  """
  @spec hkdf_sha256(Ockam.Vault, reference(), reference()) :: {:ok, reference()} | :error
  def hkdf_sha256(%vault_module{id: vault_id}, salt_handle, ikm_handle) do
    vault_module.hkdf_sha256(vault_id, salt_handle, ikm_handle)
  end

  @doc """
    Encrypts a payload using AES-GCM.
    Returns cipher_text after an encryption.
  """
  @spec aead_aes_gcm_encrypt(
          Ockam.Vault,
          reference(),
          non_neg_integer(),
          String.t() | binary,
          binary | String.t()
        ) :: {:ok, binary} | :error
  def aead_aes_gcm_encrypt(%vault_module{id: vault_id}, key_handle, nonce, ad, plain_text) do
    vault_module.aead_aes_gcm_encrypt(vault_id, key_handle, nonce, ad, plain_text)
  end

  @doc """
    Decrypts a payload using AES-GCM.
    Returns decrypted payload.
  """
  @spec aead_aes_gcm_decrypt(
          Ockam.Vault,
          reference(),
          non_neg_integer(),
          binary | String.t(),
          binary
        ) :: {:ok, binary | String.t()} | :error
  def aead_aes_gcm_decrypt(%vault_module{id: vault_id}, key_handle, nonce, ad, cipher_text) do
    vault_module.aead_aes_gcm_decrypt(vault_id, key_handle, nonce, ad, cipher_text)
  end

  @doc """
    Deinitializes the specified ockam vault object.
  """
  @spec deinit(Ockam.Vault) :: :ok | :error
  def deinit(%vault_module{id: vault_id}) do
    vault_module.deinit(vault_id)
  end
end
