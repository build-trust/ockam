#include "common.h"
#include <memory.h>

bool extern_error_has_error(const ockam_vault_extern_error_t* error) {
    return error->code != 0;
}

ERL_NIF_TERM ok_void(ErlNifEnv *env) {
    return enif_make_atom(env, "ok");
}

ERL_NIF_TERM ok(ErlNifEnv *env, ERL_NIF_TERM result) {
    ERL_NIF_TERM id = enif_make_atom(env, "ok");
    return enif_make_tuple2(env, id, result);
}

ERL_NIF_TERM err(ErlNifEnv *env, const char* msg) {
    ERL_NIF_TERM e = enif_make_atom(env, "error");
    ERL_NIF_TERM m = enif_make_string(env, msg, 0);
    return enif_make_tuple2(env, e, m);
}

int parse_vault_handle(ErlNifEnv *env, ERL_NIF_TERM argv, ockam_vault_t* vault) {
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

    ErlNifUInt64 vault_type = 0;
    enif_get_uint64(env, head, &vault_type);
    vault->handle = handle;
    vault->vault_type = vault_type;

    return 0;
}