#include "erl_nif.h"
#include "ockam/vault.h"
#include "stdint.h"
#include "string.h"

static const size_t MAX_ARG_STR_SIZE = 32;

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
    if (0 != argc) {
        return enif_make_badarg(env);
    }

    ockam_vault_t vault;

    if (0 != ockam_vault_default_init(&vault)) {
        return err(env, "failed to create vault connection");
    }

    ERL_NIF_TERM vault_handle = enif_make_uint64(env, vault);

    return ok(env, vault_handle);
}

static ERL_NIF_TERM sha256(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (2 != argc) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 vault;
    if (0 == enif_get_uint64(env, argv[0], &vault)) {
        return enif_make_badarg(env);
    }

    ErlNifBinary input;
    if (0 == enif_inspect_binary(env, argv[1], &input)) {
        return enif_make_badarg(env);
    }

    ERL_NIF_TERM term;
    uint8_t* digest = enif_make_new_binary(env, 32, &term);

    if (NULL == digest) {
        return err(env, "failed to create buffer for hash");
    }

    memset(digest, 0, 32);

    if (0 != ockam_vault_sha256(vault, input.data, input.size, digest)) {
        return err(env,  "failed to compute sha256 digest");
    }

    return ok(env, term);
}

static ERL_NIF_TERM random_bytes(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (2 != argc) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 vault;
    if (0 == enif_get_uint64(env, argv[0], &vault)) {
        return enif_make_badarg(env);
    }

    uint32_t size;
    if (0 == enif_get_uint(env, argv[1], &size)) {
        return enif_make_badarg(env);
    }

    ERL_NIF_TERM term;
    uint8_t* bytes = enif_make_new_binary(env, size, &term);

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
    if (2 != argc) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 vault;
    if (0 == enif_get_uint64(env, argv[0], &vault)) {
        return enif_make_badarg(env);
    }

    size_t num_keys;
    if (0 == enif_get_map_size(env, argv[1], &num_keys)) {
        return enif_make_badarg(env);
    }

    if (3 != num_keys) {
        return enif_make_badarg(env);
    }

    ockam_vault_secret_attributes_t attributes;

    ERL_NIF_TERM term = enif_make_atom(env, "type");
    ERL_NIF_TERM value;

    if (0 == enif_get_map_value(env, argv[1], term, &value)) {
        enif_make_badarg(env);
    }

    char buf[MAX_ARG_STR_SIZE]; // TODO: Document max allowed size somewhere?
    ERL_NIF_TERM result = enif_get_atom(env, value, buf, sizeof(buf), ERL_NIF_LATIN1);

    if (0 == result) {
        return enif_make_badarg(env);
    }

    // TODO: Document hardcoded values somewhere?
    if (strncmp("buffer", buf, sizeof(buf)) == 0) {
        attributes.type = OCKAM_VAULT_SECRET_TYPE_BUFFER;
    } else if (strncmp("aes128", buf, sizeof(buf)) == 0) {
        attributes.type = OCKAM_VAULT_SECRET_TYPE_AES128_KEY;
    } else if (strncmp("aes256", buf, sizeof(buf)) == 0) {
        attributes.type = OCKAM_VAULT_SECRET_TYPE_AES256_KEY;
    } else if (strncmp("curve25519", buf, sizeof(buf)) == 0) {
        attributes.type = OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY;
    } else if (strncmp("p256", buf, sizeof(buf)) == 0) {
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

    result = enif_get_atom(env, value, buf, sizeof(buf), ERL_NIF_LATIN1);

    if (0 == result) {
        return enif_make_badarg(env);
    }

    if (strncmp("ephemeral", buf, sizeof(buf)) == 0) {
        attributes.persistence = OCKAM_VAULT_SECRET_EPHEMERAL;
    } else if (strncmp("persistent", buf, sizeof(buf)) == 0) {
        attributes.persistence = OCKAM_VAULT_SECRET_PERSISTENT;
    } else {
        return enif_make_badarg(env);
    }

    term = enif_make_atom(env, "purpose");

    result = enif_get_map_value(env, argv[1], term, &value);

    if (0 == result) {
        return enif_make_badarg(env);
    }

    result = enif_get_atom(env, value, buf, sizeof(buf), ERL_NIF_LATIN1);

    if (0 == result) {
        return enif_make_badarg(env);
    }

    if (strncmp("keyagreement", buf, sizeof(buf)) == 0) {
        attributes.purpose = OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT;
    } else {
        return enif_make_badarg(env);
    }

    ockam_vault_secret_t secret;
    if (0 != ockam_vault_secret_generate(vault, &secret, attributes)) {
        return err(env, "unable to generate the secret");
    }

    ERL_NIF_TERM secret_handle = enif_make_uint64(env, secret.handle);

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
