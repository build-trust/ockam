/**
 * @file  stdlib.h
 * @brief
 */

#ifndef OCKAM_MEMORY_STDLIB_H_
#define OCKAM_MEMORY_STDLIB_H_

#include "ockam/error.h"
#include "ockam/memory.h"

#include "ockam/memory/impl.h"

extern const char* const OCKAM_MEMORY_STDLIB_ERROR_DOMAIN;

typedef enum {
  OCKAM_MEMORY_STDLIB_ERROR_INVALID_PARAM = 1,
  OCKAM_MEMORY_STDLIB_ERROR_INVALID_SIZE  = 2,
  OCKAM_MEMORY_STDLIB_ERROR_ALLOC_FAIL    = 3,
} ockam_error_code_memory_stdlib_t;

/**
 * @brief   Initialize the standard library memory object
 * @param   memory[in]  The ockam memory object to initialize.
 * @return  OCKAM_ERROR_NONE on success.
 * @return  MEMORY_ERROR_INVALID_PARAM if invalid memory pointer is received.
 */
ockam_error_t ockam_memory_stdlib_init(ockam_memory_t* memory);

#endif
