#include "erl_nif.h"
#include "ockam/vault.h"
#include "string.h"

static ERL_NIF_TERM default_init(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    int result = 0;
    ERL_NIF_TERM vault_handle;
    ERL_NIF_TERM vault_id;
    ockam_vault_t vault;

    result = ockam_vault_default_init(&vault);
    if (0 != result) {
        return enif_make_int(env, 0);
    }

    vault_handle = enif_make_int64(env, vault.handle);
    vault_id = enif_make_int(env, vault.vault_id);

    return enif_make_tuple2(env, vault_handle, vault_id);
}

static ERL_NIF_TERM sha256(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
  int result = 0;
  const ERL_NIF_TERM* array;
  int arity;
  unsigned char* digest;
  unsigned int size;
  ErlNifSInt64 handle;
  int vault_id;
  ockam_vault_t vault;
  ErlNifBinary input;
  ERL_NIF_TERM term;

  if (0 == array) {
      return enif_make_badarg(env);
  }

  result = enif_get_tuple(env, argv[0], &arity, &array);

  if (0 == result || arity != 2) {
      return enif_make_badarg(env);
  }

  result = enif_get_int64(env, array[0], &handle);

  if (0 == result) {
      return enif_make_badarg(env);
  }

  vault.handle = handle;

  result = enif_get_int(env, array[1], &vault_id);

  if (0 == result) {
      return enif_make_badarg(env);
  }

  vault.vault_id = vault_id;

  result = enif_inspect_binary(env, argv[1], &input);

  if (0 == result) {
      return enif_make_badarg(env);
  }

  digest = enif_make_new_binary(env, 32, &term);

  if (0 == digest) {
      return enif_make_atom(env, "null");
  }

  memset(digest, 0, 32);

  result = ockam_test(vault.vault_id, 3);
  return enif_make_uint(env, result);
//  result = ockam_vault_sha256(vault, input.data, 4, digest);

//  if (0 != result) {
//      return enif_make_atom(env, "null");
//  }
//
//  return term;
}

static ErlNifFunc nifs[] = {
  // {erl_function_name, erl_function_arity, c_function}
  {"default_init", 0, default_init},
  {"sha256", 2, sha256}
};

ERL_NIF_INIT(Elixir.Ockam.Vault.Software, nifs, NULL, NULL, NULL, NULL)
