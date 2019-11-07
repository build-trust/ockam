#include <stdlib.h>
#include "ockam/vault.h"

struct ockam_vault_t {
  int n;
};

ockam_vault_t ockam_vault_init() {
  ockam_vault_t vault;
  vault = malloc(sizeof(ockam_vault_t));
  vault->n = 60;
  return vault;
}

int ockam_vault_random(ockam_vault_t vault) {
  int n;
  n = vault->n;
  return n;
}

void ockam_vault_free(ockam_vault_t *vault) {
  free(vault);
}
