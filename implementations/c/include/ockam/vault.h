#ifndef OCKAM_VAULT_H
#define OCKAM_VAULT_H

typedef struct ockam_vault_t *ockam_vault_t;
typedef struct ockam_vault_t *ockam_vault_t;


extern ockam_vault_t ockam_vault_init(void);
extern int ockam_vault_random(ockam_vault_t vault);
extern void ockam_vault_free(ockam_vault_t *vault);

#endif
