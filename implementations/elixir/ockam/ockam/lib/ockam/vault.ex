defmodule Ockam.Vault do
  @moduledoc false

  def sha256(%vault_module{id: vault_id}, b) do
    vault_module.sha256(vault_id, b)
  end

  def random_bytes(%vault_module{id: vault_id}, b) do
    vault_module.random_bytes(vault_id, b)
  end

  def secret_generate(%vault_module{id: vault_id}, b) do
    vault_module.secret_generate(vault_id, b)
  end

  def secret_import(%vault_module{id: vault_id}, b, c) do
    vault_module.secret_import(vault_id, b, c)
  end

  def secret_export(%vault_module{id: vault_id}, b) do
    vault_module.secret_export(vault_id, b)
  end

  def secret_publickey_get(%vault_module{id: vault_id}, b) do
    vault_module.secret_publickey_get(vault_id, b)
  end

  def secret_attributes_get(%vault_module{id: vault_id}, b) do
    vault_module.secret_attributes_get(vault_id, b)
  end

  def secret_destroy(%vault_module{id: vault_id}, b) do
    vault_module.secret_destroy(vault_id, b)
  end

  def ecdh(%vault_module{id: vault_id}, b, c) do
    vault_module.ecdh(vault_id, b, c)
  end

  def hkdf_sha256(%vault_module{id: vault_id}, b, c, d) do
    vault_module.hkdf_sha256(vault_id, b, c, d)
  end

  def hkdf_sha256(%vault_module{id: vault_id}, b, c) do
    vault_module.hkdf_sha256(vault_id, b, c)
  end

  def aead_aes_gcm_encrypt(%vault_module{id: vault_id}, b, c, d, e) do
    vault_module.aead_aes_gcm_encrypt(vault_id, b, c, d, e)
  end

  def aead_aes_gcm_decrypt(%vault_module{id: vault_id}, b, c, d, e) do
    vault_module.aead_aes_gcm_decrypt(vault_id, b, c, d, e)
  end

  def deinit(%vault_module{id: vault_id}) do
    vault_module.deinit(vault_id)
  end
end
