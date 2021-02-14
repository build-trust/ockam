#include "erl_nif.h"
#include "vault.h"

static ErlNifFunc nifs[] = {
  // {erl_function_name, erl_function_arity, c_function}
  {"default_init", 0, default_init},
  {"sha256", 2, sha256},
  {"secret_generate", 2, secret_generate},
  {"secret_import", 3, secret_import},
  {"secret_export", 2, secret_export},
  {"secret_publickey_get", 2, secret_publickey_get},
  {"secret_attributes_get", 2, secret_attributes_get},
  {"secret_destroy", 2, secret_destroy},
  {"ecdh", 3, ecdh},
  {"hkdf_sha256", 3, hkdf_sha256},
  {"hkdf_sha256", 4, hkdf_sha256},
  {"aead_aes_gcm_encrypt", 5, aead_aes_gcm_encrypt},
  {"aead_aes_gcm_decrypt", 5, aead_aes_gcm_decrypt},
  {"deinit", 1, deinit},
};

ERL_NIF_INIT(Elixir.Ockam.Vault.Software, nifs, NULL, NULL, NULL, NULL)
