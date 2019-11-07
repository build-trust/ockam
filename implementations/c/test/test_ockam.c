#include <stdio.h>
#include "ockam/vault.h"

int main(int argc, char const *argv[]) {
  ockam_vault_t v = ockam_vault_init();
  int r = ockam_vault_random(v);

  if(r == 60) {
    printf("PASSED.\n");
    return 0;
  } else {
    printf("FAILED.\n");
    return 1;
  }
}
