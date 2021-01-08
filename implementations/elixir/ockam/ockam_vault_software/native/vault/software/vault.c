#include <memory.h>
#include "common.h"
#include "vault.h"
#include "ockam/vault.h"

static const size_t MAX_ARG_STR_SIZE         = 32;
static const size_t MAX_SECRET_EXPORT_SIZE   = 65;
static const size_t MAX_PUBLICKEY_SIZE       = 65;
static const size_t MAX_DERIVED_OUTPUT_COUNT = 2;
static const size_t MAX_PERSISTENCE_ID_SIZE  = 64;

static const char* SECRET_TYPE_KEY        = "type";
static const char* SECRET_TYPE_BUFFER     = "buffer";
static const char* SECRET_TYPE_AES        = "aes";
static const char* SECRET_TYPE_CURVE25519 = "curve25519";
static const char* SECRET_TYPE_P256       = "p256";

static const char* SECRET_PERSISTENCE_KEY        = "persistence";
static const char* SECRET_PERSISTENCE_EPHEMERAL  = "ephemeral";
static const char* SECRET_PERSISTENCE_PERSISTENT = "persistent";

static const char* SECRET_LENGTH_KEY = "length";

static int parse_secret_attributes(ErlNifEnv *env, ERL_NIF_TERM arg, ockam_vault_secret_attributes_t* attributes) {
    size_t num_keys;
    if (0 == enif_get_map_size(env, arg, &num_keys)) {
        return -1;
    }

    if (num_keys < 3 || 4 < num_keys) {
        return -1;
    }

    ERL_NIF_TERM term = enif_make_atom(env, SECRET_TYPE_KEY);
    ERL_NIF_TERM value;

    if (0 == enif_get_map_value(env, arg, term, &value)) {
        return -1;
    }

    char buf[MAX_ARG_STR_SIZE]; // TODO: Document max allowed size somewhere?

    if (0 == enif_get_atom(env, value, buf, sizeof(buf), ERL_NIF_LATIN1)) {
        return -1;
    }

    if (strncmp(SECRET_TYPE_BUFFER, buf, sizeof(buf)) == 0) {
        attributes->type = OCKAM_VAULT_SECRET_TYPE_BUFFER;
    } else if (strncmp(SECRET_TYPE_AES, buf, sizeof(buf)) == 0) {
        attributes->type = OCKAM_VAULT_SECRET_TYPE_AES_KEY;
    } else if (strncmp(SECRET_TYPE_CURVE25519, buf, sizeof(buf)) == 0) {
        attributes->type = OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY;
    } else if (strncmp(SECRET_TYPE_P256, buf, sizeof(buf)) == 0) {
        attributes->type = OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY;
    } else {
        return -1;
    }

    term = enif_make_atom(env, SECRET_PERSISTENCE_KEY);

    // FIXME: Replace with iterator
    if (0 == enif_get_map_value(env, arg, term, &value)) {
        return -1;
    }

    if (0 == enif_get_atom(env, value, buf, sizeof(buf), ERL_NIF_LATIN1)) {
        return -1;
    }

    if (strncmp(SECRET_PERSISTENCE_EPHEMERAL, buf, sizeof(buf)) == 0) {
        attributes->persistence = OCKAM_VAULT_SECRET_EPHEMERAL;
    } else if (strncmp(SECRET_PERSISTENCE_PERSISTENT, buf, sizeof(buf)) == 0) {
        attributes->persistence = OCKAM_VAULT_SECRET_PERSISTENT;
    } else {
        return -1;
    }

    term = enif_make_atom(env, SECRET_LENGTH_KEY);

    uint32_t length = 0;
    if (0 != enif_get_map_value(env, arg, term, &value)) {
        if (0 == enif_get_uint(env, value, &length)) {
            return -1;
        }
    }

    attributes->length = length;

    return 0;
}

static ERL_NIF_TERM create_term_from_secret_attributes(ErlNifEnv *env, const ockam_vault_secret_attributes_t* attributes) {
    ERL_NIF_TERM map = enif_make_new_map(env);

    ERL_NIF_TERM type_key = enif_make_atom(env, SECRET_TYPE_KEY);

    const char* type_value_str;
    switch (attributes->type) {
        case OCKAM_VAULT_SECRET_TYPE_BUFFER: type_value_str = SECRET_TYPE_BUFFER; break;
        case OCKAM_VAULT_SECRET_TYPE_AES_KEY: type_value_str = SECRET_TYPE_AES; break;
        case OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY: type_value_str = SECRET_TYPE_CURVE25519; break;
        case OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY: type_value_str = SECRET_TYPE_P256; break;

        default:
            return enif_make_badarg(env);
    }

    ERL_NIF_TERM type_value = enif_make_atom(env, type_value_str);

    if (0 == enif_make_map_put(env, map, type_key, type_value, &map)) {
        return enif_make_badarg(env);
    }

    ERL_NIF_TERM persistence_key = enif_make_atom(env, SECRET_PERSISTENCE_KEY);

    const char* persistence_value_str;
    switch (attributes->persistence) {
        case OCKAM_VAULT_SECRET_EPHEMERAL: persistence_value_str = SECRET_PERSISTENCE_EPHEMERAL; break;
        case OCKAM_VAULT_SECRET_PERSISTENT: persistence_value_str = SECRET_PERSISTENCE_PERSISTENT; break;

        default:
            return enif_make_badarg(env);
    }

    ERL_NIF_TERM persistence_value = enif_make_atom(env, persistence_value_str);

    if (0 == enif_make_map_put(env, map, persistence_key, persistence_value, &map)) {
        return enif_make_badarg(env);
    }

    ERL_NIF_TERM length_key = enif_make_atom(env, SECRET_LENGTH_KEY);
    ERL_NIF_TERM length_value = enif_make_uint(env, attributes->length);

    if (0 == enif_make_map_put(env, map, length_key, length_value, &map)) {
        return enif_make_badarg(env);
    }

    return ok(env, map);
}

ERL_NIF_TERM default_init(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (0 != argc) {
        return enif_make_badarg(env);
    }

    ockam_vault_t vault;

    ockam_vault_extern_error_t error = ockam_vault_default_init(&vault);
    if (extern_error_has_error(&error)) {
        return err(env, "failed to create vault connection");
    }

    ERL_NIF_TERM handle = enif_make_uint64(env, vault.handle);
    ERL_NIF_TERM vault_type = enif_make_uint64(env, vault.vault_type);

    ERL_NIF_TERM vault_handle = enif_make_list2(env, handle, vault_type);

    return ok(env, vault_handle);
}

ERL_NIF_TERM file_init(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (1 != argc) {
        return enif_make_badarg(env);
    }

    ockam_vault_t vault;

    ErlNifBinary file;
    if (0 == enif_inspect_binary(env, argv[0], &file)) {
        return enif_make_badarg(env);
    }
    ERL_NIF_TERM term;
    uint8_t* buffer = enif_make_new_binary(env, file.size+1, &term);

    if (NULL == buffer) {
        return err(env, "failed to create path buffer");
    }

    memset(buffer, 0, file.size+1);
    memcpy(buffer, file.data, file.size);

    ockam_vault_extern_error_t error = ockam_vault_file_init(&vault, buffer);
    if (extern_error_has_error(&error)) {
        return err(env, "failed to create vault connection");
    }

    ERL_NIF_TERM handle = enif_make_uint64(env, vault.handle);
    ERL_NIF_TERM vault_type = enif_make_uint64(env, vault.vault_type);

    ERL_NIF_TERM vault_handle = enif_make_list2(env, handle, vault_type);

    return ok(env, vault_handle);
}

ERL_NIF_TERM sha256(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (2 != argc) {
        return enif_make_badarg(env);
    }

    ockam_vault_t vault;
    if (0 != parse_vault_handle(env, argv[0], &vault)) {
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

    ockam_vault_extern_error_t error = ockam_vault_sha256(vault, input.data, input.size, digest);
    if (extern_error_has_error(&error)) {
        return err(env,  "failed to compute sha256 digest");
    }

    return ok(env, term);
}

ERL_NIF_TERM secret_generate(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (2 != argc) {
        return enif_make_badarg(env);
    }

    ockam_vault_t vault;
    if (0 != parse_vault_handle(env, argv[0], &vault)) {
        return enif_make_badarg(env);
    }

    ockam_vault_secret_attributes_t attributes;
    if (0 != parse_secret_attributes(env, argv[1], &attributes)) {
        return enif_make_badarg(env);
    }

    ockam_vault_secret_t secret;
    ockam_vault_extern_error_t error = ockam_vault_secret_generate(vault, &secret, attributes);
    if (extern_error_has_error(&error)) {
        return err(env, "unable to generate the secret");
    }

    ERL_NIF_TERM secret_handle = enif_make_uint64(env, secret);

    return ok(env, secret_handle);
}

ERL_NIF_TERM secret_import(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (3 != argc) {
        return enif_make_badarg(env);
    }

    ockam_vault_t vault;
    if (0 != parse_vault_handle(env, argv[0], &vault)) {
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
    ockam_vault_extern_error_t error = ockam_vault_secret_import(vault, &secret, attributes, input.data, input.size);
    if (extern_error_has_error(&error)) {
        return err(env, "unable to import the secret");
    }

    ERL_NIF_TERM secret_handle = enif_make_uint64(env, secret);

    return ok(env, secret_handle);
}

ERL_NIF_TERM secret_export(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (2 != argc) {
        return enif_make_badarg(env);
    }

    ockam_vault_t vault;
    if (0 != parse_vault_handle(env, argv[0], &vault)) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 secret_handle;
    if (0 == enif_get_uint64(env, argv[1], &secret_handle)) {
        return enif_make_badarg(env);
    }

    uint8_t buffer[MAX_SECRET_EXPORT_SIZE];
    size_t length = 0;

    ockam_vault_extern_error_t error = ockam_vault_secret_export(vault, secret_handle, buffer, MAX_SECRET_EXPORT_SIZE, &length);
    if (extern_error_has_error(&error)) {
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

ERL_NIF_TERM secret_publickey_get(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (2 != argc) {
        return enif_make_badarg(env);
    }

    ockam_vault_t vault;
    if (0 != parse_vault_handle(env, argv[0], &vault)) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 secret_handle;
    if (0 == enif_get_uint64(env, argv[1], &secret_handle)) {
        return enif_make_badarg(env);
    }

    uint8_t buffer[MAX_PUBLICKEY_SIZE];
    size_t length = 0;

    ockam_vault_extern_error_t error = ockam_vault_secret_publickey_get(vault, secret_handle, buffer, MAX_SECRET_EXPORT_SIZE, &length);
    if (extern_error_has_error(&error)) {
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

ERL_NIF_TERM secret_attributes_get(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (2 != argc) {
        return enif_make_badarg(env);
    }

    ockam_vault_t vault;
    if (0 != parse_vault_handle(env, argv[0], &vault)) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 secret_handle;
    if (0 == enif_get_uint64(env, argv[1], &secret_handle)) {
        return enif_make_badarg(env);
    }

    ockam_vault_secret_attributes_t attributes;
    ockam_vault_extern_error_t error = ockam_vault_secret_attributes_get(vault, secret_handle, &attributes);
    if (extern_error_has_error(&error)) {
        return err(env, "failed to secret_attributes_get");
    }

    return create_term_from_secret_attributes(env, &attributes);
}

ERL_NIF_TERM secret_destroy(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (2 != argc) {
        return enif_make_badarg(env);
    }

    ockam_vault_t vault;
    if (0 != parse_vault_handle(env, argv[0], &vault)) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 secret_handle;
    if (0 == enif_get_uint64(env, argv[1], &secret_handle)) {
        return enif_make_badarg(env);
    }

    ockam_vault_extern_error_t error = ockam_vault_secret_destroy(vault, secret_handle);
    if (extern_error_has_error(&error)) {
        return err(env, "failed to secret_destroy");
    }

    return ok_void(env);
}

ERL_NIF_TERM ecdh(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (3 != argc) {
        return enif_make_badarg(env);
    }

    ockam_vault_t vault;
    if (0 != parse_vault_handle(env, argv[0], &vault)) {
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
    ockam_vault_extern_error_t error = ockam_vault_ecdh(vault, secret_handle, input.data,  input.size, &shared_secret);
    if (extern_error_has_error(&error)) {
        return err(env, "failed to ecdh");
    }

    ERL_NIF_TERM shared_secret_term = enif_make_uint64(env, shared_secret);

    return ok(env, shared_secret_term);
}

ERL_NIF_TERM hkdf_sha256(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (4 != argc && 3 != argc) {
        return enif_make_badarg(env);
    }

    ockam_vault_t vault;
    if (0 != parse_vault_handle(env, argv[0], &vault)) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 salt_handle;
    if (0 == enif_get_uint64(env, argv[1], &salt_handle)) {
        return enif_make_badarg(env);
    }

    size_t i = 2;
    ErlNifUInt64 ikm_handle;
    const ockam_vault_secret_t *ikm_handle_ptr = (const ockam_vault_secret_t *) &ikm_handle;
    if (argc == 4) {
        if (0 == enif_get_uint64(env, argv[2], &ikm_handle)) {
            return enif_make_badarg(env);
        }
        i++;
    }
    else {
        ikm_handle_ptr = NULL;
    }

    unsigned int derived_outputs_count;
    if (0 == enif_get_list_length(env, argv[i], &derived_outputs_count)) {
        return enif_make_badarg(env);
    }

    if (derived_outputs_count > MAX_DERIVED_OUTPUT_COUNT) {
        return enif_make_badarg(env);
    }

    ockam_vault_secret_attributes_t attributes[MAX_DERIVED_OUTPUT_COUNT];
    ockam_vault_secret_attributes_t* attr = attributes;

    ERL_NIF_TERM current_list = argv[i];
    ERL_NIF_TERM head;
    ERL_NIF_TERM tail;

    for (unsigned int j = 0; j < derived_outputs_count; j++, attr++) {
        if (0 == enif_get_list_cell(env, current_list, &head, &tail)) {
            return enif_make_badarg(env);
        }
        current_list = tail;
        if (0 != parse_secret_attributes(env, head, attr)) {
            return enif_make_badarg(env);
        }
    }

    ockam_vault_secret_t shared_secrets[MAX_DERIVED_OUTPUT_COUNT];
    ockam_vault_extern_error_t error = ockam_vault_hkdf_sha256(vault, salt_handle, ikm_handle_ptr, attributes, derived_outputs_count, shared_secrets);
    if (extern_error_has_error(&error)) {
        return err(env, "failed to hkdf_sha256");
    }

    ERL_NIF_TERM output_array[MAX_DERIVED_OUTPUT_COUNT];
    for (size_t j = 0; j < derived_outputs_count; j++) {
        output_array[j] = enif_make_uint64(env, shared_secrets[j]);
    }

    ERL_NIF_TERM output = enif_make_list_from_array(env, output_array, derived_outputs_count);

    return ok(env, output);
}

ERL_NIF_TERM aead_aes_gcm_encrypt(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (5 != argc) {
        return enif_make_badarg(env);
    }

    ockam_vault_t vault;
    if (0 != parse_vault_handle(env, argv[0], &vault)) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 key_handle;
    if (0 == enif_get_uint64(env, argv[1], &key_handle)) {
        return enif_make_badarg(env);
    }

    unsigned int nonce;
    if (0 == enif_get_uint(env, argv[2], &nonce)) {
        return enif_make_badarg(env);
    }

    ErlNifBinary ad;
    if (0 == enif_inspect_binary(env, argv[3], &ad)) {
        return enif_make_badarg(env);
    }

    ErlNifBinary plain_text;
    if (0 == enif_inspect_binary(env, argv[4], &plain_text)) {
        return enif_make_badarg(env);
    }

    ERL_NIF_TERM term;
    // FIXME: Allocated size should be provided by the rust lib
    size_t size = plain_text.size + 16;
    uint8_t* cipher_text = enif_make_new_binary(env, size, &term);

    if (NULL == cipher_text) {
        return err(env, "failed to create buffer for aead_aes_gcm_encrypt");
    }

    memset(cipher_text, 0, size);

    size_t length = 0;

    ockam_vault_extern_error_t error = ockam_vault_aead_aes_gcm_encrypt(vault,
                                                                        key_handle,
                                                                        nonce,
                                                                        ad.data,
                                                                        ad.size,
                                                                        plain_text.data,
                                                                        plain_text.size,
                                                                        cipher_text,
                                                                        size,
                                                                        &length);
    if (extern_error_has_error(&error)) {
        return err(env, "failed to aead_aes_gcm_encrypt");
    }

    if (length != size) {
        return err(env, "buffer size is invalid during aead_aes_gcm_encrypt");
    }

    return ok(env, term);
}

ERL_NIF_TERM aead_aes_gcm_decrypt(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (5 != argc) {
        return enif_make_badarg(env);
    }

    ockam_vault_t vault;
    if (0 != parse_vault_handle(env, argv[0], &vault)) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 key_handle;
    if (0 == enif_get_uint64(env, argv[1], &key_handle)) {
        return enif_make_badarg(env);
    }

    unsigned int nonce;
    if (0 == enif_get_uint(env, argv[2], &nonce)) {
        return enif_make_badarg(env);
    }

    ErlNifBinary ad;
    if (0 == enif_inspect_binary(env, argv[3], &ad)) {
        return enif_make_badarg(env);
    }

    ErlNifBinary cipher_text;
    if (0 == enif_inspect_binary(env, argv[4], &cipher_text)) {
        return enif_make_badarg(env);
    }

    if (cipher_text.size < 16) {
        return enif_make_badarg(env);
    }

    ERL_NIF_TERM term;
    // FIXME: Allocated size should be provided by the rust lib
    size_t size = cipher_text.size - 16;
    uint8_t* plain_text = enif_make_new_binary(env, size, &term);

    if (NULL == plain_text) {
        return err(env, "failed to create buffer for aead_aes_gcm_decrypt");
    }

    memset(plain_text, 0, size);

    size_t length = 0;

    ockam_vault_extern_error_t error = ockam_vault_aead_aes_gcm_decrypt(vault,
                                                                        key_handle,
                                                                        nonce,
                                                                        ad.data,
                                                                        ad.size,
                                                                        cipher_text.data,
                                                                        cipher_text.size,
                                                                        plain_text,
                                                                        size,
                                                                        &length);
    if (extern_error_has_error(&error)) {
        return err(env, "failed to aead_aes_gcm_decrypt");
    }

    if (length != size) {
        return err(env, "buffer size is invalid during aead_aes_gcm_decrypt");
    }

    return ok(env, term);
}

ERL_NIF_TERM get_persistence_id(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (2 != argc) {
        return enif_make_badarg(env);
    }

    ockam_vault_t vault;
    if (0 != parse_vault_handle(env, argv[0], &vault)) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 key_handle;
    if (0 == enif_get_uint64(env, argv[1], &key_handle)) {
        return enif_make_badarg(env);
    }

    char persistence_id[MAX_PERSISTENCE_ID_SIZE];

    ockam_vault_extern_error_t error = ockam_vault_get_persistence_id(vault, key_handle, persistence_id, sizeof(persistence_id));
    if (extern_error_has_error(&error)) {
        return err(env, "failed to ockam_vault_get_persistence_id");
    }

    return ok(env, enif_make_string(env, persistence_id, ERL_NIF_LATIN1));
}

ERL_NIF_TERM get_persistent_secret(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (2 != argc) {
        return enif_make_badarg(env);
    }

    ockam_vault_t vault;
    if (0 != parse_vault_handle(env, argv[0], &vault)) {
        return enif_make_badarg(env);
    }

    char persistence_id[MAX_PERSISTENCE_ID_SIZE];
    if (0 == enif_get_string(env, argv[1], persistence_id, sizeof(persistence_id), ERL_NIF_LATIN1)) {
        return enif_make_badarg(env);
    }

    ockam_vault_secret_t key_handle;
    ockam_vault_extern_error_t error = ockam_vault_get_persistent_secret(vault, &key_handle, persistence_id);
    if (extern_error_has_error(&error)) {
        return err(env, "failed to ockam_vault_get_persistent_secret");
    }

    ERL_NIF_TERM secret_handle = enif_make_uint64(env, key_handle);

    return ok(env, secret_handle);
}

ERL_NIF_TERM deinit(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (1 != argc) {
        return enif_make_badarg(env);
    }

    ockam_vault_t vault;
    if (0 != parse_vault_handle(env, argv[0], &vault)) {
        return enif_make_badarg(env);
    }

    ockam_vault_extern_error_t error = ockam_vault_deinit(vault);
    if (extern_error_has_error(&error)) {
        return err(env, "failed to deinit vault");
    }

    return ok_void(env);
}