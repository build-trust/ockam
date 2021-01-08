#ifndef OCKAM_ELIXIR_KEX_H
#define OCKAM_ELIXIR_KEX_H

#include "erl_nif.h"

ERL_NIF_TERM xx_initiator(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]);
ERL_NIF_TERM xx_responder(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]);
ERL_NIF_TERM process(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]);
ERL_NIF_TERM is_complete(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]);
ERL_NIF_TERM finalize(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]);

#endif //OCKAM_ELIXIR_KEX_H
