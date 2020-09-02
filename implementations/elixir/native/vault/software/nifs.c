#include "erl_nif.h"
#include "ockam/vault.h"
#include "stdint.h"
#include "string.h"

static ERL_NIF_TERM ok(ErlNifEnv *env, ERL_NIF_TERM result) {
    ERL_NIF_TERM id = enif_make_atom(env, "ok");
    return enif_make_tuple2(env, id, result);
}

static ERL_NIF_TERM err(ErlNifEnv *env, const char* msg) {
    ERL_NIF_TERM e = enif_make_atom(env, "error");
    ERL_NIF_TERM m = enif_make_string(env, msg, 0);
    return enif_make_tuple2(env, e, m);
}

static ERL_NIF_TERM default_init(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    ERL_NIF_TERM vault_handle;
    ockam_vault_t vault;

    if (0 != ockam_vault_default_init(&vault)) {
        return err(env, "failed to create vault connection");
    }

    vault_handle = enif_make_uint64(env, vault);

    return ok(env, vault_handle);
}

static ERL_NIF_TERM sha256(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    ErlNifUInt64 vault;
    uint8_t* digest;
    ErlNifBinary input;
    ERL_NIF_TERM term;

    if (0 == enif_get_uint64(env, argv[0], &vault)) {
        return enif_make_badarg(env);
    }

    if (0 == enif_inspect_binary(env, argv[1], &input)) {
        return enif_make_badarg(env);
    }

    digest = enif_make_new_binary(env, 32, &term);

    if (0 == digest) {
        return err(env, "failed to create buffer for hash");
    }

    memset(digest, 0, 32);

    if (0 != ockam_vault_sha256(vault, input.data, input.size, digest)) {
        return err(env,  "failed to compute sha256 digest");
    }

    return ok(env, term);
}

static ERL_NIF_TERM random_bytes(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    ErlNifUInt64 vault;
    uint8_t* bytes;
    uint32_t size;
    ERL_NIF_TERM term;

    if (0 == enif_get_uint64(env, argv[0], &vault)) {
        return enif_make_badarg(env);
    }

    if (0 == enif_get_uint(env, argv[1], &size)) {
        return enif_make_badarg(env);
    }

    bytes = enif_make_new_binary(env, size, &term);

    if (0 == bytes) {
        return err(env, "failed to create buffer for random bytes");
    }

    memset(bytes, 0, size);
    if (0 != ockam_vault_random_bytes_generate(vault, bytes, size)) {
        return err(env, "failed to generate random bytes");
    }

    return ok(env, term);
}

static ERL_NIF_TERM secret_generate(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    ErlNifUInt64 vault;
    ockam_vault_secret_t secret;
    ockam_vault_secret_attributes_t attributes;
    size_t num_keys;
    char* buf;
    unsigned size;
    ERL_NIF_TERM term;
    ERL_NIF_TERM value;
    ERL_NIF_TERM secret_handle;
    ERL_NIF_TERM result;

    if (0 == enif_get_uint64(env, argv[0], &vault)) {
        return enif_make_badarg(env);
    }
    if (0 == enif_get_map_size(env, argv[1], &num_keys)) {
        return enif_make_badarg(env);
    }

    if (3 != num_keys) {
        return enif_make_badarg(env);
    }

    term = enif_make_atom(env, "type");

    if (0 == enif_get_map_value(env, argv[1], term, &value)) {
        enif_make_badarg(env);
    }

    result = enif_get_atom(env, value, buf, size, 0);

    if (0 == result || size == 0) {
        return enif_make_badarg(env);
    }

    if (strcmp("buffer", buf) == 0) {
        attributes.type = OCKAM_VAULT_SECRET_TYPE_BUFFER;
    } else if (strcmp("aes128", buf) == 0) {
        attributes.type = OCKAM_VAULT_SECRET_TYPE_AES128_KEY;
    } else if (strcmp("aes256", buf) == 0) {
        attributes.type = OCKAM_VAULT_SECRET_TYPE_AES256_KEY;
    } else if (strcmp("curve25519", buf) == 0) {
        attributes.type = OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY;
    } else if (strcmp("p256", buf) == 0) {
        attributes.type = OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY;
    } else {
        return enif_make_badarg(env);
    }

    term = enif_make_atom(env, "persistence");

    result = enif_get_map_value(env, argv[1], term, &value);

    if (0 == result) {
        enif_make_badarg(env);
        return enif_make_uint64(env, 0);
    }

    result = enif_get_atom(env, value, buf, size, 0);

    if (0 == result) {
        return enif_make_badarg(env);
    }

    if (strcmp("ephemeral", buf) == 0) {
        attributes.persistence = OCKAM_VAULT_SECRET_EPHEMERAL;
    } else if (strcmp("persistent", buf) == 0) {
        attributes.persistence = OCKAM_VAULT_SECRET_PERSISTENT;
    } else {
        enif_make_badarg(env);
    }

    term = enif_make_atom(env, "purpose");

    result = enif_get_map_value(env, argv[1], term, &value);

    if (0 == result) {
        return enif_make_badarg(env);
    }

    result = enif_get_atom(env, value, buf, size, 0);

    if (0 == result) {
        return enif_make_badarg(env);
    }

    if (strcmp("keyagreement", buf) == 0) {
        attributes.purpose = OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT;
    } else {
        return enif_make_badarg(env);
    }

    if (0 != ockam_vault_secret_generate(vault, &secret, attributes)) {
        return err(env, "unable to generate the secret");
    }

    // TODO: convert to TERM

    return ok(env, secret_handle);
}

static ErlNifFunc nifs[] = {
  // {erl_function_name, erl_function_arity, c_function}
  {"default_init", 0, default_init},
  {"random_bytes", 2, random_bytes},
  {"sha256", 2, sha256},
  {"secret_generate", 2, secret_generate},
};

ERL_NIF_INIT(Elixir.Ockam.Vault.Software, nifs, NULL, NULL, NULL, NULL)
