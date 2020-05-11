/**
 * @file  random.c
 * @brief
 */

#include "ockam/error.h"
#include "ockam/random.h"

#include "random/impl.h"

ockam_error_t ockam_random_deinit(ockam_random_t* random)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((random == 0) || (random->dispatch == 0)) {
    error = OCKAM_RANDOM_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = random->dispatch->deinit(random);

exit:
  return error;
}

ockam_error_t ockam_random_get_bytes(ockam_random_t* random, uint8_t* buffer, size_t buffer_size)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((random == 0) || (random->dispatch == 0)) {
    error = OCKAM_RANDOM_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = random->dispatch->get_bytes(random, buffer, buffer_size);

exit:
  return error;
}
