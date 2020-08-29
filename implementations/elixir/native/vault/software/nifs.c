#include "erl_nif.h"
#include "ockam/vault.h"
#include "stdint.h"
#include "string.h"

static const char* ATT_TY[] = {"t", "y" };
static const char* ATT_PERSISTENT[] = {"p", "e", "r", "s", "i", "s", "t", "e", "n", "c", "e" };
static const char* ATT_PURPOSE[] = {"p", "u", "r", "p", "o", "s", "e" };

static int32_t get_vault(ErlNifEnv *env, const ERL_NIF_TERM argv[], ockam_vault_t* vault);

static ERL_NIF_TERM default_init(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    int32_t result = 0;
    ERL_NIF_TERM vault_handle;
    ERL_NIF_TERM vault_id;
    ockam_vault_t vault;

    result = ockam_vault_default_init(&vault);
    if (0 != result) {
        return enif_make_int(env, 0);
    }

    vault_handle = enif_make_uint64(env, vault.handle);
    vault_id = enif_make_uint(env, vault.vault_id);

    return enif_make_tuple2(env, vault_handle, vault_id);
}

static ERL_NIF_TERM sha256(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
  int32_t result = 0;
  ockam_vault_t vault;
  uint8_t* digest;
  size_t size;
  ErlNifBinary input;
  ERL_NIF_TERM term;

  result = get_vault(env, argv, &vault);
  if (0 == result) {
      return enif_make_int(env, 0);
  }

  result = enif_inspect_binary(env, argv[1], &input);

  if (0 == result) {
      return enif_make_badarg(env);
  }

  digest = enif_make_new_binary(env, 32, &term);

  if (0 == digest) {
      return enif_make_atom(env, "null");
  }

  memset(digest, 0, 32);

  result = ockam_vault_sha256(vault, input.data, input.size, digest);

  if (0 != result) {
      return enif_make_atom(env, "null");
  }

  return term;
}

static ERL_NIF_TERM random_bytes(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    int32_t result = 0;
    ockam_vault_t vault;
    uint8_t* bytes;
    uint32_t size;
    ErlNifBinary input;
    ERL_NIF_TERM term;

    result = get_vault(env, argv, &vault);
    if (0 == result) {
        return enif_make_int(env, 0);
    }

    result = enif_get_uint(env, argv[1], &size);

    if (0 == result) {
        return enif_make_badarg(env);
    }

    bytes = enif_make_new_binary(env, size, &term);

    if (0 == bytes) {
        return enif_make_atom(env, "null");
    }

    memset(bytes, 0, size);
    result = ockam_vault_random_bytes_generate(vault, bytes, size);

    if (0 != result) {
        return enif_make_atom(env, "null");
    }

    return term;
}

static ERL_NIF_TERM secret_generate(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    int32_t result = 0;
    ockam_vault_t vault;
    ockam_vault_secret_t secret;
    ockam_vault_secret_attributes_t attributes;
    size_t num_keys;
    ERL_NIF_TERM term;
    ERL_NIF_TERM value;
    ERL_NIF_TERM secret_handle;
    int e;

    result = get_vault(env, argv, &vault);
    if (0 == result) {
        return enif_make_uint64(env, 0);
    }

    result = enif_get_map_size(env, argv[1], &num_keys);
    if (0 == result) {
        enif_make_badarg(env);
        return enif_make_uint64(env, 0);
    }

    if (3 != num_keys) {
        enif_make_badarg(env);
        return enif_make_uint64(env, 0);
    }

    term = enif_make_atom(env, "type");

    result = enif_get_map_value(env, argv[1], term, &value);

    if (0 == result) {
        enif_make_badarg(env);
        return enif_make_uint64(env, 0);
    }

    result = enif_get_int(env, value, &e);

    if (0 == result) {
        enif_make_badarg(env);
        return enif_make_uint64(env, 0);
    }

    if (0 <= e && e <= 4) {
        attributes.type = (ockam_vault_secret_type_t)e;
    } else {
        enif_make_badarg(env);
        return enif_make_uint64(env, 0);
    }

    term = enif_make_atom(env, "persistence");

    result = enif_get_map_value(env, argv[1], term, &value);

    if (0 == result) {
        enif_make_badarg(env);
        return enif_make_uint64(env, 0);
    }

    result = enif_get_int(env, value, &e);

    if (0 == result) {
        enif_make_badarg(env);
        return enif_make_uint64(env, 0);
    }

    if (0 <= e && e <= 1) {
        attributes.persistence = (ockam_vault_secret_persistence_t)e;
    } else {
        enif_make_badarg(env);
        return enif_make_uint64(env, 0);
    }

    term = enif_make_atom(env, "purpose");

    result = enif_get_map_value(env, argv[1], term, &value);

    if (0 == result) {
        enif_make_badarg(env);
        return enif_make_uint64(env, 0);
    }

    result = enif_get_int(env, value, &e);

    if (0 == result) {
        enif_make_badarg(env);
        return enif_make_uint64(env, 0);
    }

    if (0 == e) {
        attributes.purpose = (ockam_vault_secret_purpose_t)e;
    } else {
        enif_make_badarg(env);
        return enif_make_uint64(env, 0);
    }

    result = ockam_vault_secret_generate(vault, &secret, attributes);

    if (0 != result) {
        return enif_make_uint64(env, 0);
    }

    return enif_make_uint64(env, secret.handle);
}

static int32_t get_vault(ErlNifEnv *env, const ERL_NIF_TERM argv[], ockam_vault_t* vault) {
    int result;
    int arity;
    const ERL_NIF_TERM* array;
    ErlNifUInt64 handle;
    unsigned int vault_id;

    if (0 == array) {
        enif_make_badarg(env);
        return 0;
    }

    result = enif_get_tuple(env, argv[0], &arity, &array);

    if (0 == result || arity != 2) {
        enif_make_badarg(env);
        return 0;
    }

    result = enif_get_uint64(env, array[0], &handle);

    if (0 == result) {
        enif_make_badarg(env);
        return 0;
    }

    vault->handle = handle;

    result = enif_get_uint(env, array[1], &vault_id);

    if (0 == result) {
        enif_make_badarg(env);
        return 0;
    }

    vault->vault_id = vault_id;

    return 1;
}

static ErlNifFunc nifs[] = {
  // {erl_function_name, erl_function_arity, c_function}
  {"default_init", 0, default_init},
  {"random_bytes", 2, random_bytes},
  {"sha256", 2, sha256},
  {"secret_generate", 2, secret_generate},
};

ERL_NIF_INIT(Elixir.Ockam.Vault.Software, nifs, NULL, NULL, NULL, NULL)
