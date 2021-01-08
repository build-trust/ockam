#include "erl_nif.h"
#include "vault.h"
#include "kex.h"

static ErlNifFunc nifs[] = {
  // {erl_function_name, erl_function_arity, c_function}
  {"default_init", 0, default_init},
  {"file_init", 1, file_init},
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
  {"get_persistence_id", 2, get_persistence_id},
  {"get_persistent_secret", 2, get_persistent_secret},
  {"deinit", 1, deinit},
  {"xx_initiator", 2, xx_initiator},
  {"xx_responder", 2, xx_responder},
  {"process", 2, process},
  {"is_complete", 1, is_complete},
  {"finalize", 1, finalize},
};

ERL_NIF_INIT(Elixir.Ockam.Vault.Software, nifs, NULL, NULL, NULL, NULL)
