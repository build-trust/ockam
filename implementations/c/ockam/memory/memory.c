/**
 * @file  memory.c
 * @brief
 */

#include "ockam/error.h"
#include "ockam/memory.h"

#include "ockam/memory/impl.h"

const char* const OCKAM_MEMORY_INTERFACE_ERROR_DOMAIN = "OCKAM_MEMORY_INTERFACE_ERROR_DOMAIN";

static const ockam_error_t ockam_memory_interface_error_none = {
  OCKAM_ERROR_NONE,
  OCKAM_MEMORY_INTERFACE_ERROR_DOMAIN
};

ockam_error_t ockam_memory_deinit(ockam_memory_t* memory)
{
  ockam_error_t error = ockam_memory_interface_error_none;

  if ((memory == 0) || (memory->dispatch == 0)) {
    error.code = OCKAM_MEMORY_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = memory->dispatch->deinit(memory);

exit:
  return error;
}

ockam_error_t ockam_memory_alloc_zeroed(ockam_memory_t* memory, void** buffer, size_t buffer_size)
{
  ockam_error_t error = ockam_memory_interface_error_none;

  if ((memory == 0) || (memory->dispatch == 0)) {
    error.code = OCKAM_MEMORY_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = memory->dispatch->alloc_zeroed(memory, buffer, buffer_size);

exit:
  return error;
}

ockam_error_t ockam_memory_free(ockam_memory_t* memory, void* buffer, size_t buffer_size)
{
  ockam_error_t error = ockam_memory_interface_error_none;

  if ((memory == 0) || (memory->dispatch == 0)) {
    error.code = OCKAM_MEMORY_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = memory->dispatch->free(memory, buffer, buffer_size);

exit:
  return error;
}

ockam_error_t ockam_memory_copy(ockam_memory_t* memory, void* destination, const void* source, size_t copy_size)
{
  ockam_error_t error = ockam_memory_interface_error_none;

  if ((memory == 0) || (memory->dispatch == 0)) {
    error.code = OCKAM_MEMORY_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = memory->dispatch->copy(memory, destination, source, copy_size);

exit:
  return error;
}

ockam_error_t ockam_memory_set(ockam_memory_t* memory, void* buffer, uint8_t value, size_t set_size)
{
  ockam_error_t error = ockam_memory_interface_error_none;

  if ((memory == 0) || (memory->dispatch == 0)) {
    error.code = OCKAM_MEMORY_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = memory->dispatch->set(memory, buffer, value, set_size);

exit:
  return error;
}

ockam_error_t ockam_memory_move(ockam_memory_t* memory, void* destination, void* source, size_t move_size)
{
  ockam_error_t error = ockam_memory_interface_error_none;

  if ((memory == 0) || (memory->dispatch == 0)) {
    error.code = OCKAM_MEMORY_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = memory->dispatch->move(memory, destination, source, move_size);

exit:
  return error;
}

ockam_error_t ockam_memory_compare(ockam_memory_t* memory, int *res, const void* lhs, const void* rhs, size_t move_size)
{
  ockam_error_t error = ockam_memory_interface_error_none;

  if ((memory == 0) || (memory->dispatch == 0)) {
    error.code = OCKAM_MEMORY_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = memory->dispatch->compare(memory, res, lhs, rhs, move_size);

exit:
  return error;
}
