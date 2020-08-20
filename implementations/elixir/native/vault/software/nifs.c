#include "erl_nif.h"

static ERL_NIF_TERM sha256(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
  int a, b;
  enif_get_int(env, argv[0], &a);
  enif_get_int(env, argv[1], &b);

  int result = a + b;
  return enif_make_int(env, result);
}

static ErlNifFunc nifs[] = {
  // {erl_function_name, erl_function_arity, c_function}
  {"sha256", 2, sha256}
};

ERL_NIF_INIT(Elixir.Ockam.Vault.Software, nifs, NULL, NULL, NULL, NULL)
