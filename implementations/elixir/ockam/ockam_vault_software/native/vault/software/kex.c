#include <memory.h>
#include <stdbool.h>
#include "ockam/kex.h"
#include "kex.h"
#include "common.h"

static const size_t MAX_RESPONSE_SIZE = 1024; // FIXME
static const char* H_KEY              = "h";
static const char* ENCRYPT_KEY        = "encrypt_key";
static const char* DECRYPT_KEY        = "decrypt_key";
static const char* PUBLIC_KEY         = "public_key";

int parse_kex_handle(ErlNifEnv *env, ERL_NIF_TERM argv, ockam_kex_t* kex) {
    unsigned int count;
    if (0 == enif_get_list_length(env, argv, &count)) {
        return -1;
    }

    if (count != 2) {
        return -1;
    }

    ERL_NIF_TERM current_list = argv;
    ERL_NIF_TERM head;
    ERL_NIF_TERM tail;

    if (0 == enif_get_list_cell(env, current_list, &head, &tail)) {
        return -1;
    }
    current_list = tail;

    ErlNifUInt64 handle = 0;
    enif_get_uint64(env, head, &handle);

    if (0 == enif_get_list_cell(env, current_list, &head, &tail)) {
        return -1;
    }

    ErlNifUInt64 kex_type = 0;
    enif_get_uint64(env, head, &kex_type);
    kex->handle = handle;
    kex->kex_type = kex_type;

    return 0;
}

ERL_NIF_TERM xx_initiator(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
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

    ockam_kex_t kex;

    ockam_vault_extern_error_t error = ockam_kex_xx_initiator(&kex, vault, secret_handle);
    if (extern_error_has_error(&error)) {
        return err(env, "failed to create xx initiator");
    }

    ERL_NIF_TERM handle = enif_make_uint64(env, kex.handle);
    ERL_NIF_TERM kex_type = enif_make_uint64(env, kex.kex_type);

    ERL_NIF_TERM kex_handle = enif_make_list2(env, handle, kex_type);

    return ok(env, kex_handle);
}

ERL_NIF_TERM xx_responder(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
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

    ockam_kex_t kex;

    ockam_vault_extern_error_t error = ockam_kex_xx_responder(&kex, vault, secret_handle);
    if (extern_error_has_error(&error)) {
        return err(env, "failed to create xx responder");
    }

    ERL_NIF_TERM handle = enif_make_uint64(env, kex.handle);
    ERL_NIF_TERM kex_type = enif_make_uint64(env, kex.kex_type);

    ERL_NIF_TERM kex_handle = enif_make_list2(env, handle, kex_type);

    return ok(env, kex_handle);
}

ERL_NIF_TERM process(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (2 != argc) {
        return enif_make_badarg(env);
    }

    ockam_kex_t kex;
    if (0 != parse_kex_handle(env, argv[0], &kex)) {
        return enif_make_badarg(env);
    }

    ErlNifBinary input;
    if (0 == enif_inspect_binary(env, argv[1], &input)) {
        return enif_make_badarg(env);
    }

    uint8_t buffer[MAX_RESPONSE_SIZE];
    size_t length = 0;

    ockam_vault_extern_error_t error = ockam_kex_process(kex, input.data, input.size, buffer, MAX_RESPONSE_SIZE, &length);
    if (extern_error_has_error(&error)) {
        return err(env, "failed to ockam_kex_process");
    }

    ERL_NIF_TERM output;
    uint8_t* bytes = enif_make_new_binary(env, length, &output);

    if (0 == bytes) {
        return err(env, "failed to create buffer for ockam_kex_process");
    }
    memcpy(bytes, buffer, length);

    return ok(env, output);
}

ERL_NIF_TERM is_complete(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    ockam_kex_t kex;
    if (0 != parse_kex_handle(env, argv[0], &kex)) {
        return enif_make_badarg(env);
    }

    bool is_complete = false;
    ockam_vault_extern_error_t error = ockam_kex_is_complete(kex, &is_complete);
    if (extern_error_has_error(&error)) {
        return err(env, "failed to ockam_kex_is_complete");
    }

    return ok(env, enif_make_int(env, is_complete ? 1 : 0));
}

static ERL_NIF_TERM create_term_from_completed_key_exchange(ErlNifEnv *env, const ockam_completed_key_exchange_t* completed_key_exchange) {
    ERL_NIF_TERM map = enif_make_new_map(env);

    ERL_NIF_TERM h_key = enif_make_atom(env, H_KEY);
    ERL_NIF_TERM h_value;
    uint8_t* h_bytes = enif_make_new_binary(env, sizeof(completed_key_exchange->h), &h_value);

    if (0 == h_bytes) {
        return err(env, "failed to create buffer for create_term_from_completed_key_exchange");
    }
    memcpy(h_bytes, completed_key_exchange->h, sizeof(completed_key_exchange->h));

    if (0 == enif_make_map_put(env, map, h_key, h_value, &map)) {
        return enif_make_badarg(env);
    }

    ERL_NIF_TERM encrypt_key = enif_make_atom(env, ENCRYPT_KEY);
    if (0 == enif_make_map_put(env, map, encrypt_key, enif_make_uint64(env, completed_key_exchange->encrypt_key), &map)) {
        return enif_make_badarg(env);
    }

    ERL_NIF_TERM decrypt_key = enif_make_atom(env, DECRYPT_KEY);
    if (0 == enif_make_map_put(env, map, decrypt_key, enif_make_uint64(env, completed_key_exchange->decrypt_key), &map)) {
        return enif_make_badarg(env);
    }

    ERL_NIF_TERM pub_key = enif_make_atom(env, PUBLIC_KEY);
    ERL_NIF_TERM pub_value;
    uint8_t* pub_bytes = enif_make_new_binary(env, sizeof(completed_key_exchange->h), &pub_value);

    if (0 == pub_value) {
        return err(env, "failed to create buffer for create_term_from_completed_key_exchange");
    }
    memcpy(pub_bytes, completed_key_exchange->remote_static_public_key, completed_key_exchange->remote_static_public_key_len);

    if (0 == enif_make_map_put(env, map, pub_key, pub_value, &map)) {
        return enif_make_badarg(env);
    }

    return ok(env, map);
}

ERL_NIF_TERM finalize(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (1 != argc) {
        return enif_make_badarg(env);
    }

    ockam_kex_t kex;
    if (0 != parse_kex_handle(env, argv[0], &kex)) {
        return enif_make_badarg(env);
    }

    ockam_completed_key_exchange_t completed_key_exchange;
    ockam_vault_extern_error_t error = ockam_kex_finalize(kex, &completed_key_exchange);
    if (extern_error_has_error(&error)) {
        return err(env, "failed to ockam_kex_finalize");
    }

    return create_term_from_completed_key_exchange(env, &completed_key_exchange);
}
