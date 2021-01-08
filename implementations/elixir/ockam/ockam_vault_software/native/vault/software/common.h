#ifndef OCKAM_ELIXIR_COMMON_H
#define OCKAM_ELIXIR_COMMON_H

#include <memory.h>
#include <stdbool.h>
#include <ockam/vault.h>
#include "erl_nif.h"

bool extern_error_has_error(const ockam_vault_extern_error_t* error);

ERL_NIF_TERM ok_void(ErlNifEnv *env);

ERL_NIF_TERM ok(ErlNifEnv *env, ERL_NIF_TERM result);

ERL_NIF_TERM err(ErlNifEnv *env, const char* msg);

int parse_vault_handle(ErlNifEnv *env, ERL_NIF_TERM argv, ockam_vault_t* vault);

#endif //OCKAM_ELIXIR_COMMON_H
