/**
 * @file  memory.c
 * @brief
 */

#include "ockam/error.h"
#include "ockam/memory.h"

#include "memory/impl.h"

ockam_error_t ockam_memory_deinit(ockam_memory_t* memory)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((memory == 0) || (memory->dispatch == 0)) {
    error = OCKAM_MEMORY_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = memory->dispatch->deinit(memory);

exit:
  return error;
}

ockam_error_t ockam_memory_alloc_zeroed(ockam_memory_t* memory, void** buffer, size_t buffer_size)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((memory == 0) || (memory->dispatch == 0)) {
    error = OCKAM_MEMORY_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = memory->dispatch->alloc_zeroed(memory, buffer, buffer_size);

exit:
  return error;
}

ockam_error_t ockam_memory_free(ockam_memory_t* memory, void* buffer, size_t buffer_size)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((memory == 0) || (memory->dispatch == 0)) {
    error = OCKAM_MEMORY_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = memory->dispatch->free(memory, buffer, buffer_size);

exit:
  return error;
}

ockam_error_t ockam_memory_copy(ockam_memory_t* memory, void* destination, const void* source, size_t copy_size)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((memory == 0) || (memory->dispatch == 0)) {
    error = OCKAM_MEMORY_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = memory->dispatch->copy(memory, destination, source, copy_size);

exit:
  return error;
}

ockam_error_t ockam_memory_set(ockam_memory_t* memory, void* buffer, uint8_t value, size_t set_size)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((memory == 0) || (memory->dispatch == 0)) {
    error = OCKAM_MEMORY_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = memory->dispatch->set(memory, buffer, value, set_size);

exit:
  return error;
}

ockam_error_t ockam_memory_move(ockam_memory_t* memory, void* destination, void* source, size_t move_size)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((memory == 0) || (memory->dispatch == 0)) {
    error = OCKAM_MEMORY_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = memory->dispatch->move(memory, destination, source, move_size);

exit:
  return error;
}
