#ifndef XX_H
#define XX_H

#include "ockam/error.h"
#include "ockam/memory.h"
#include "ockam/vault.h"
#include "ockam/io.h"

#include "ockam/key_agreement/impl.h"

extern const char* const OCKAM_KEY_AGREEMENT_XX_ERROR_DOMAIN;

typedef enum {
  OCKAM_KEY_AGREEMENT_XX_ERROR_INVALID_PARAM = 1,
  OCKAM_KEY_AGREEMENT_XX_ERROR_SMALL_BUFFER  = 2,
  OCKAM_KEY_AGREEMENT_XX_ERROR_FAIL          = 3,
} ockam_error_code_key_agreement_xx_t;

extern const ockam_error_t ockam_key_agreement_xx_error_none;

ockam_error_t ockam_xx_key_initialize(ockam_key_t* key, ockam_memory_t* memory, ockam_vault_t* vault);

#endif
