#include "erl_nif.h"
#include "ockam/vault.h"

static ERL_NIF_TERM sha256(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
  int result = 0;

  ockam_vault_t vault;
  result = ockam_vault_default_init(&vault);
  if (0 != result) {
    return enif_make_int(env, 1);
  }

  return enif_make_int(env, result);
}

static ErlNifFunc nifs[] = {
  // {erl_function_name, erl_function_arity, c_function}
  {"sha256", 2, sha256}
};

ERL_NIF_INIT(Elixir.Ockam.Vault.Software, nifs, NULL, NULL, NULL, NULL)
