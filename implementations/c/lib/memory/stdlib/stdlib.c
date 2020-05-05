/**
 * @file    stdlib.c
 * @brief   impl of Ockam's memory functions using stdlib calls
 */

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "ockam/error.h"
#include "ockam/memory.h"

#include "memory/stdlib/stdlib.h"

ockam_error_t memory_stdlib_deinit(ockam_memory_t* memory);
ockam_error_t memory_stdlib_alloc(ockam_memory_t* memory, uint8_t** buffer, size_t buffer_size);
ockam_error_t memory_stdlib_free(ockam_memory_t* memory, uint8_t* buffer, size_t buffer_size);
ockam_error_t memory_stdlib_set(ockam_memory_t* memory, uint8_t* buffer, uint8_t value, size_t set_size);
ockam_error_t memory_stdlib_copy(ockam_memory_t* memory, uint8_t* destination, const uint8_t* source, size_t copy_size);
ockam_error_t memory_stdlib_move(ockam_memory_t* memory, uint8_t* destination, uint8_t* source, size_t move_size);

ockam_memory_dispatch_table_t memory_stdlib_dispatch_table = { &memory_stdlib_deinit, &memory_stdlib_alloc,
                                                               &memory_stdlib_free,   &memory_stdlib_set,
                                                               &memory_stdlib_copy,   &memory_stdlib_move };

ockam_error_t ockam_memory_stdlib_init(ockam_memory_t* memory)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if (memory == 0) {
    error = MEMORY_ERROR_INVALID_PARAM;
    goto exit;
  }

  memory->dispatch = &memory_stdlib_dispatch_table;
  memory->context  = 0;

exit:
  return error;
}

ockam_error_t memory_stdlib_deinit(ockam_memory_t* memory)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

exit:
  return error;
}

ockam_error_t memory_stdlib_alloc(ockam_memory_t* memory, uint8_t** buffer, size_t buffer_size)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((memory == 0) || (buffer == 0)) {
    error = MEMORY_ERROR_INVALID_PARAM;
    goto exit;
  }

  if (buffer_size == 0) {
    error = MEMORY_ERROR_INVALID_SIZE;
    goto exit;
  }

  *buffer = malloc(buffer_size);

  if (*buffer == 0) {
    error = MEMORY_ERROR_ALLOC_FAIL;
    goto exit;
  }

  memset(*buffer, 0, buffer_size);

exit:
  return error;
}

ockam_error_t memory_stdlib_free(ockam_memory_t* memory, uint8_t* buffer, size_t buffer_size)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  (void) buffer_size;

  if (buffer == 0) {
    error = MEMORY_ERROR_INVALID_PARAM;
    goto exit;
  }

  free(buffer);

exit:
  return error;
}

ockam_error_t memory_stdlib_set(ockam_memory_t* memory, uint8_t* buffer, uint8_t value, size_t set_size)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((memory == 0) || (buffer == 0)) {
    error = MEMORY_ERROR_INVALID_PARAM;
    goto exit;
  }

  memset(buffer, value, set_size);

exit:
  return error;
}

ockam_error_t memory_stdlib_copy(ockam_memory_t* memory, uint8_t* destination, const uint8_t* source, size_t copy_size)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((memory == 0) || (destination == 0) || (source == 0)) {
    error = MEMORY_ERROR_INVALID_PARAM;
    goto exit;
  }

  memcpy(destination, source, copy_size);

exit:
  return error;
}

ockam_error_t memory_stdlib_move(ockam_memory_t* memory, uint8_t* destination, uint8_t* source, size_t move_size)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((memory == 0) || (destination == 0) || (source == 0)) {
    error = MEMORY_ERROR_INVALID_PARAM;
    goto exit;
  }

  memmove(destination, source, move_size);

exit:
  return error;
}
