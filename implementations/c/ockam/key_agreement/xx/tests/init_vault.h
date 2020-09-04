#ifndef OCKAM_INIT_VAULT_H
#define OCKAM_INIT_VAULT_H

#include <ockam/random.h>
#include <ockam/vault.h>

typedef enum {
    VAULT_OPT_NONE      = 0,
    VAULT_OPT_DEFAULT   = 1,
    VAULT_OPT_ATECC608A = 2,
} VAULT_OPT_t;

ockam_error_t init_vault(ockam_vault_t *vault, VAULT_OPT_t vault_opt, ockam_memory_t *memory, ockam_random_t *random);

#endif //OCKAM_INIT_VAULT_H
