#include "erl_nif.h"
/* #include "ockam.h" */

static ERL_NIF_TERM
random(ErlNifEnv *env, int argc, const ERL_NIF_TERM argv[]) {
  /* int r = rand(); */
  int r = 50;
  return enif_make_int(env, r);
}

static ErlNifFunc
nif_funcs[] = {
  {"random", 0, random, 0},
};

ERL_NIF_INIT(Elixir.Ockam, nif_funcs, NULL, NULL, NULL, NULL)
