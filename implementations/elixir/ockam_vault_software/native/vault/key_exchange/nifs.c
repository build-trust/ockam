#include <string.h>
#include "erl_nif.h"
#include "ockam/kex.h"

// FIXME: Allocate memory chunk of exact size before each encode
static const size_t MAX_KEX_MESSAGE_SIZE = 1024;

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

static ERL_NIF_TERM kex_init_initiator(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (1 != argc) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 vault_handle;
    if (0 == enif_get_uint64(env, argv[0], &vault_handle)) {
        return enif_make_badarg(env);
    }

    ockam_kex_initiator_t initiator_handle;

    if (0 != ockam_kex_xx_initiator(&initiator_handle, vault_handle)) {
        return err(env, "failed to kex_init_initiator");
    }

    ERL_NIF_TERM kex_handle_term = enif_make_uint64(env, initiator_handle);

    return ok(env, kex_handle_term);
}

static ERL_NIF_TERM kex_init_responder(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (1 != argc) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 vault_handle;
    if (0 == enif_get_uint64(env, argv[0], &vault_handle)) {
        return enif_make_badarg(env);
    }

    ockam_kex_responder_t responder_handle;

    if (0 != ockam_kex_xx_responder(&responder_handle, vault_handle)) {
        return err(env, "failed to kex_init_responder");
    }

    ERL_NIF_TERM kex_handle_term = enif_make_uint64(env, responder_handle);

    return ok(env, kex_handle_term);
}

static ERL_NIF_TERM kex_initiator_encode_message_1(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (2 != argc) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 initiator_handle;
    if (0 == enif_get_uint64(env, argv[0], &initiator_handle)) {
        return enif_make_badarg(env);
    }

    ErlNifBinary payload;
    if (0 == enif_inspect_binary(env, argv[1], &payload)) {
        return enif_make_badarg(env);
    }

    uint8_t buffer[MAX_KEX_MESSAGE_SIZE];
    size_t length = 0;

    if (0 != ockam_kex_xx_initiator_encode_message_1(initiator_handle,
                                                     payload.data,
                                                     payload.size,
                                                     buffer,
                                                     MAX_KEX_MESSAGE_SIZE,
                                                     &length)) {
        return err(env, "failed to kex_initiator_encode_message_1");
    }

    ERL_NIF_TERM output;
    uint8_t* bytes = enif_make_new_binary(env, length, &output);

    if (0 == bytes) {
        return err(env, "failed to create buffer for kex_initiator_encode_message_1");
    }
    memcpy(bytes, buffer, length);

    return ok(env, output);
}

static ERL_NIF_TERM kex_responder_encode_message_2(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (2 != argc) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 responder_handle;
    if (0 == enif_get_uint64(env, argv[0], &responder_handle)) {
        return enif_make_badarg(env);
    }

    ErlNifBinary payload;
    if (0 == enif_inspect_binary(env, argv[1], &payload)) {
        return enif_make_badarg(env);
    }

    uint8_t buffer[MAX_KEX_MESSAGE_SIZE];
    size_t length = 0;

    if (0 != ockam_kex_xx_responder_encode_message_2(responder_handle,
                                                     payload.data,
                                                     payload.size,
                                                     buffer,
                                                     MAX_KEX_MESSAGE_SIZE,
                                                     &length)) {
        return err(env, "failed to kex_responder_encode_message_2");
    }

    ERL_NIF_TERM output;
    uint8_t* bytes = enif_make_new_binary(env, length, &output);

    if (0 == bytes) {
        return err(env, "failed to create buffer for kex_responder_encode_message_2");
    }
    memcpy(bytes, buffer, length);

    return ok(env, output);
}

static ERL_NIF_TERM kex_initiator_encode_message_3(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (2 != argc) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 initiator_handle;
    if (0 == enif_get_uint64(env, argv[0], &initiator_handle)) {
        return enif_make_badarg(env);
    }

    ErlNifBinary payload;
    if (0 == enif_inspect_binary(env, argv[1], &payload)) {
        return enif_make_badarg(env);
    }

    uint8_t buffer[MAX_KEX_MESSAGE_SIZE];
    size_t length = 0;

    if (0 != ockam_kex_xx_initiator_encode_message_3(initiator_handle,
                                                     payload.data,
                                                     payload.size,
                                                     buffer,
                                                     MAX_KEX_MESSAGE_SIZE,
                                                     &length)) {
        return err(env, "failed to kex_initiator_encode_message_3");
    }

    ERL_NIF_TERM output;
    uint8_t* bytes = enif_make_new_binary(env, length, &output);

    if (0 == bytes) {
        return err(env, "failed to create buffer for kex_initiator_encode_message_3");
    }
    memcpy(bytes, buffer, length);

    return ok(env, output);
}

static ERL_NIF_TERM kex_responder_decode_message_1(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (2 != argc) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 responder_handle;
    if (0 == enif_get_uint64(env, argv[0], &responder_handle)) {
        return enif_make_badarg(env);
    }

    ErlNifBinary m1;
    if (0 == enif_inspect_binary(env, argv[1], &m1)) {
        return enif_make_badarg(env);
    }

    if (0 != ockam_kex_xx_responder_decode_message_1(responder_handle, m1.data, m1.size)) {
        return err(env, "failed to kex_responder_decode_message_1");
    }

    return ok_void(env);
}

static ERL_NIF_TERM kex_initiator_decode_message_2(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (2 != argc) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 initiator_handle;
    if (0 == enif_get_uint64(env, argv[0], &initiator_handle)) {
        return enif_make_badarg(env);
    }

    ErlNifBinary m2;
    if (0 == enif_inspect_binary(env, argv[1], &m2)) {
        return enif_make_badarg(env);
    }

    if (0 != ockam_kex_xx_initiator_decode_message_2(initiator_handle, m2.data, m2.size)) {
        return err(env, "failed to kex_initiator_decode_message_2");
    }

    return ok_void(env);
}

static ERL_NIF_TERM kex_responder_decode_message_3(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (2 != argc) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 responder_handle;
    if (0 == enif_get_uint64(env, argv[0], &responder_handle)) {
        return enif_make_badarg(env);
    }

    ErlNifBinary m3;
    if (0 == enif_inspect_binary(env, argv[1], &m3)) {
        return enif_make_badarg(env);
    }

    if (0 != ockam_kex_xx_responder_decode_message_3(responder_handle, m3.data, m3.size)) {
        return err(env, "failed to kex_responder_decode_message_3");
    }

    return ok_void(env);
}

static ERL_NIF_TERM kex_initiator_finalize(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (1 != argc) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 initiator_handle;
    if (0 == enif_get_uint64(env, argv[0], &initiator_handle)) {
        return enif_make_badarg(env);
    }

    ockam_kex_t kex;

    if (0 != ockam_kex_xx_initiator_finalize(initiator_handle, &kex)) {
        return err(env, "failed to kex_initiator_finalize");
    }

    ERL_NIF_TERM kex_handle_term = enif_make_uint64(env, kex);

    return ok(env, kex_handle_term);
}

static ERL_NIF_TERM kex_responder_finalize(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
    if (1 != argc) {
        return enif_make_badarg(env);
    }

    ErlNifUInt64 responder_handle;
    if (0 == enif_get_uint64(env, argv[0], &responder_handle)) {
        return enif_make_badarg(env);
    }

    ockam_kex_t kex;

    if (0 != ockam_kex_xx_responder_finalize(responder_handle, &kex)) {
        return err(env, "failed to ockam_kex_xx_responder_finalize");
    }

    ERL_NIF_TERM kex_handle_term = enif_make_uint64(env, kex);

    return ok(env, kex_handle_term);
}


static ErlNifFunc nifs[] = {
  // {erl_function_name, erl_function_arity, c_function}
  {"kex_init_initiator", 1, kex_init_initiator},
  {"kex_init_responder", 1, kex_init_responder},
  {"kex_initiator_encode_message_1", 2, kex_initiator_encode_message_1},
  {"kex_responder_encode_message_2", 2, kex_responder_encode_message_2},
  {"kex_initiator_encode_message_3", 2, kex_initiator_encode_message_3},
  {"kex_responder_decode_message_1", 2, kex_responder_decode_message_1},
  {"kex_initiator_decode_message_2", 2, kex_initiator_decode_message_2},
  {"kex_responder_decode_message_3", 2, kex_responder_decode_message_3},
  {"kex_initiator_finalize", 1, kex_initiator_finalize},
  {"kex_responder_finalize", 1, kex_responder_finalize},
};

ERL_NIF_INIT(Elixir.Ockam.Kex.Rust, nifs, NULL, NULL, NULL, NULL)
