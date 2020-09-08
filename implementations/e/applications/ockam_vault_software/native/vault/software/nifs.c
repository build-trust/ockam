#include "erl_nif.h"
#include "ockam/vault.h"
#include "stdint.h"
#include "string.h"

static const size_t MAX_ARG_STR_SIZE       = 32;
static const size_t MAX_SECRET_EXPORT_SIZE = 65;
static const size_t MAX_PUBLICKEY_SIZE     = 65;

static ERL_NIF_TERM ok_void(ErlNifEnv *env) {
    return enif_make_atom(env, "ok");
}

static ERL_NIF_TERM ok(ErlNifEnv *env, ERL_NIF_TERM result) {
    ERL_NIF_TERM id = enif_make_atom(env, "ok");
    return enif_make_tuple2(env, id, result);
}

static ERL_NIF_TERM err(ErlNifEnv *env, const char* msg) {
    ERL_NIF_TERM e = enif_make_atom(env, "error");
    ERL_NIF_TERM m = enif_make_string(env, msg, 0);
    return enif_make_tuple2(env, e, m);
}

static const char* SECRET_TYPE_KEY        = "type";
static const char* SECRET_TYPE_BUFFER     = "buffer";
static const char* SECRET_TYPE_AES128     = "aes128";
static const char* SECRET_TYPE_AES256     = "aes256";
static const char* SECRET_TYPE_CURVE25519 = "curve25519";
static const char* SECRET_TYPE_P256       = "p256";

static const char* SECRET_PERSISTENCE_KEY        = "persistence";
static const char* SECRET_PERSISTENCE_EPHEMERAL  = "ephemeral";
static const char* SECRET_PERSISTENCE_PERSISTENT = "persistent";

static const char* SECRET_PURPOSE_KEY           = "purpose";
static const char* SECRET_PURPOSE_KEY_AGREEMENT = "key_agreement";

static int parse_secret_attributes(ErlNifEnv *env, ERL_NIF_TERM arg, ockam_vault_secret_attributes_t* attributes) {
    size_t num_keys;
    if (0 == enif_get_map_size(env, arg, &num_keys)) {
        return -1;
    }

    if (3 != num_keys) {
        return -1;
    }

    ERL_NIF_TERM term = enif_make_atom(env, SECRET_TYPE_KEY);
    ERL_NIF_TERM value;

    if (0 == enif_get_map_value(env, arg, term, &value)) {
        return -1;
    }

    char buf[MAX_ARG_STR_SIZE]; // TODO: Document max allowed size somewhere?
    ERL_NIF_TERM result = enif_get_atom(env, value, buf, sizeof(buf), ERL_NIF_LATIN1);

    if (0 == result) {
        return -1;
    }

    // TODO: Document hardcoded values somewhere?
    if (strncmp(SECRET_TYPE_BUFFER, buf, sizeof(buf)) == 0) {
        attributes->type = OCKAM_VAULT_SECRET_TYPE_BUFFER;
    } else if (strncmp(SECRET_TYPE_AES128, buf, sizeof(buf)) == 0) {
        attributes->type = OCKAM_VAULT_SECRET_TYPE_AES128_KEY;
    } else if (strncmp(SECRET_TYPE_AES256, buf, sizeof(buf)) == 0) {
        attributes->type = OCKAM_VAULT_SECRET_TYPE_AES256_KEY;
    } else if (strncmp(SECRET_TYPE_CURVE25519, buf, sizeof(buf)) == 0) {
        attributes->type = OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY;
    } else if (strncmp(SECRET_TYPE_P256, buf, sizeof(buf)) == 0) {
        attributes->type = OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY;
    } else {
        return -1;
    }

    term = enif_make_atom(env, SECRET_PERSISTENCE_KEY);

    // FIXME: Replace with iterator
    result = enif_get_map_value(env, arg, term, &value);

    if (0 == result) {
        return -1;
    }

    result = enif_get_atom(env, value, buf, sizeof(buf), ERL_NIF_LATIN1);

    if (0 == result) {
        return -1;
    }

    if (strncmp(SECRET_PERSISTENCE_EPHEMERAL, buf, sizeof(buf)) == 0) {
        attributes->persistence = OCKAM_VAULT_SECRET_EPHEMERAL;
    } else if (strncmp(SECRET_PERSISTENCE_PERSISTENT, buf, sizeof(buf)) == 0) {
        attributes->persistence = OCKAM_VAULT_SECRET_PERSISTENT;
    } else {
        return -1;
    }

    term = enif_make_atom(env, SECRET_PURPOSE_KEY);

    result = enif_get_map_value(env, arg, term, &value);

    if (0 == result) {
        return -1;
    }

    result = enif_get_atom(env, value, buf, sizeof(buf), ERL_NIF_LATIN1);

    if (0 == result) {
        return -1;
    }

    if (strncmp(SECRET_PURPOSE_KEY_AGREEMENT, buf, sizeof(buf)) == 0) {
        attributes->purpose = OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT;
    } else {
        return -1;
    }

    return 0;
}

static ERL_NIF_TERM create_term_from_secret_attributes(ErlNifEnv *env, const ockam_vault_secret_attributes_t* attributes) {
    ERL_NIF_TERM map = enif_make_new_map(env);

    ERL_NIF_TERM type_key = enif_make_atom(env, SECRET_TYPE_KEY);

    const char* type_value_str;
    switch (attributes->type) {
        case OCKAM_VAULT_SECRET_TYPE_BUFFER: type_value_str = SECRET_TYPE_BUFFER; break;
        case OCKAM_VAULT_SECRET_TYPE_AES128_KEY: type_value_str = SECRET_TYPE_AES128; break;
        case OCKAM_VAULT_SECRET_TYPE_AES256_KEY: type_value_str = SECRET_TYPE_AES256; break;
        case OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY: type_value_str = SECRET_TYPE_CURVE25519; break;
        case OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY: type_value_str = SECRET_TYPE_P256; break;

        default:
            return enif_make_badarg(env);
    }

    ERL_NIF_TERM type_value = enif_make_atom(env, type_value_str);

    enif_make_map_put(env, map, type_key, type_value, &map);

    ERL_NIF_TERM persistence_key = enif_make_atom(env, SECRET_PERSISTENCE_KEY);

    const char* persistence_value_str;
    switch (attributes->persistence) {
        case OCKAM_VAULT_SECRET_EPHEMERAL: persistence_value_str = SECRET_PERSISTENCE_EPHEMERAL; break;
        case OCKAM_VAULT_SECRET_PERSISTENT: persistence_value_str = SECRET_PERSISTENCE_PERSISTENT; break;

        default:
            return enif_make_badarg(env);
    }

    ERL_NIF_TERM persistence_value = enif_make_atom(env, persistence_value_str);

    enif_make_map_put(env, map, persistence_key, persistence_value, &map);

    ERL_NIF_TERM purpose_key = enif_make_atom(env, SECRET_PURPOSE_KEY);

    const char* purpose_value_str;
    switch (attributes->purpose) {
        case OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT: purpose_value_str = SECRET_PURPOSE_KEY_AGREEMENT; break;

        default:
            return enif_make_badarg(env);
    }

    ERL_NIF_TERM purpose_value = enif_make_atom(env, purpose_value_str);

    enif_make_map_put(env, map, purpose_key, purpose_value, &map);

    return ok(env, map);
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

    ockam_vault_secret_attributes_t attributes;
    if (0 != parse_secret_attributes(env, argv[1], &attributes)) {
        return enif_make_badarg(env);
    }

    ockam_vault_secret_t secret;
    if (0 != ockam_vault_secret_generate(vault, &secret, attributes)) {
        return err(env, "unable to generate the secret");
    }

    ERL_NIF_TERM secret_handle = enif_make_uint64(env, secret);

    return ok(env, secret_handle);
}

static ERL_NIF_TERM secret_import(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (3 != argc) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 vault;
    if (0 == enif_get_uint64(env, argv[0], &vault)) {
        return enif_make_badarg(env);
    }

    ockam_vault_secret_attributes_t attributes;
    if (0 != parse_secret_attributes(env, argv[1], &attributes)) {
        return enif_make_badarg(env);
    }

    ErlNifBinary input;
    if (0 == enif_inspect_binary(env, argv[2], &input)) {
        return enif_make_badarg(env);
    }

    ockam_vault_secret_t secret;
    if (0 != ockam_vault_secret_import(vault, &secret, attributes, input.data, input.size)) {
        return err(env, "unable to import the secret");
    }

    ERL_NIF_TERM secret_handle = enif_make_uint64(env, secret);

    return ok(env, secret_handle);
}

static ERL_NIF_TERM secret_export(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (2 != argc) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 vault;
    if (0 == enif_get_uint64(env, argv[0], &vault)) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 secret_handle;
    if (0 == enif_get_uint64(env, argv[1], &secret_handle)) {
        return enif_make_badarg(env);
    }

    uint8_t buffer[MAX_SECRET_EXPORT_SIZE];
    size_t length = 0;

    if (0 != ockam_vault_secret_export(vault, secret_handle, buffer, MAX_SECRET_EXPORT_SIZE, &length)) {
        return err(env, "failed to ockam_vault_secret_export");
    }

    ERL_NIF_TERM output;
    uint8_t* bytes = enif_make_new_binary(env, length, &output);

    if (0 == bytes) {
        return err(env, "failed to create buffer for secret export");
    }
    memcpy(bytes, buffer, length);

    return ok(env, output);
}

static ERL_NIF_TERM secret_publickey_get(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (2 != argc) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 vault;
    if (0 == enif_get_uint64(env, argv[0], &vault)) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 secret_handle;
    if (0 == enif_get_uint64(env, argv[1], &secret_handle)) {
        return enif_make_badarg(env);
    }

    uint8_t buffer[MAX_PUBLICKEY_SIZE];
    size_t length = 0;

    if (0 != ockam_vault_secret_publickey_get(vault, secret_handle, buffer, MAX_SECRET_EXPORT_SIZE, &length)) {
        return err(env, "failed to ockam_vault_secret_publickey_get");
    }

    ERL_NIF_TERM output;
    uint8_t* bytes = enif_make_new_binary(env, length, &output);

    if (0 == bytes) {
        return err(env, "failed to create buffer for secret_publickey_get");
    }
    memcpy(bytes, buffer, length);

    return ok(env, output);
}

static ERL_NIF_TERM secret_attributes_get(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (2 != argc) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 vault;
    if (0 == enif_get_uint64(env, argv[0], &vault)) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 secret_handle;
    if (0 == enif_get_uint64(env, argv[1], &secret_handle)) {
        return enif_make_badarg(env);
    }

    ockam_vault_secret_attributes_t attributes;
    if (0 != ockam_vault_secret_attributes_get(vault, secret_handle, &attributes)) {
        return err(env, "failed to secret_attributes_get");
    }

    return create_term_from_secret_attributes(env, &attributes);
}

static ERL_NIF_TERM secret_destroy(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (2 != argc) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 vault;
    if (0 == enif_get_uint64(env, argv[0], &vault)) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 secret_handle;
    if (0 == enif_get_uint64(env, argv[1], &secret_handle)) {
        return enif_make_badarg(env);
    }

    if (0 != ockam_vault_secret_destroy(vault, secret_handle)) {
        return err(env, "failed to secret_destroy");
    }

    return ok_void(env);
}

static ERL_NIF_TERM ecdh(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (3 != argc) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 vault;
    if (0 == enif_get_uint64(env, argv[0], &vault)) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 secret_handle;
    if (0 == enif_get_uint64(env, argv[1], &secret_handle)) {
        return enif_make_badarg(env);
    }

    ErlNifBinary input;
    if (0 == enif_inspect_binary(env, argv[2], &input)) {
        return enif_make_badarg(env);
    }

    ockam_vault_secret_t shared_secret;
    if (0 != ockam_vault_ecdh(vault, secret_handle, input.data,  input.size, &shared_secret)) {
        return err(env, "failed to ecdh");
    }

    ERL_NIF_TERM shared_secret_term = enif_make_uint64(env, shared_secret);

    return ok(env, shared_secret_term);
}

static ErlNifFunc nifs[] = {
  // {erl_function_name, erl_function_arity, c_function}
  {"default_init", 0, default_init},
  {"random_bytes", 2, random_bytes},
  {"sha256", 2, sha256},
  {"secret_generate", 2, secret_generate},
  {"secret_import", 3, secret_import},
  {"secret_export", 2, secret_export},
  {"secret_publickey_get", 2, secret_publickey_get},
  {"secret_attributes_get", 2, secret_attributes_get},
  {"secret_destroy", 2, secret_destroy},
  {"ecdh", 3, ecdh},
};

ERL_NIF_INIT(Elixir.Ockam.Vault.Software, nifs, NULL, NULL, NULL, NULL)
