#ifndef XX_H
#define XX_H

#include "ockam/error.h"
#include "ockam/memory.h"
#include "ockam/vault.h"
#include "ockam/io.h"

#include "ockam/key_agreement/impl.h"

ockam_error_t ockam_xx_key_initialize(
  ockam_key_t* key, ockam_memory_t* memory, ockam_vault_t* vault, ockam_reader_t* reader, ockam_writer_t* writer);

#endif
