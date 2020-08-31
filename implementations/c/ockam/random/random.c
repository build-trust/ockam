/**
 * @file  random.c
 * @brief
 */

#include "ockam/error.h"
#include "ockam/random.h"

#include "ockam/random/impl.h"

const char* const OCKAM_RANDOM_INTERFACE_ERROR_DOMAIN = "OCKAM_RANDOM_INTERFACE_ERROR_DOMAIN";

static const ockam_error_t ockam_random_interface_error_none = {
  OCKAM_ERROR_NONE,
  OCKAM_RANDOM_INTERFACE_ERROR_DOMAIN
};

ockam_error_t ockam_random_deinit(ockam_random_t* random)
{
  ockam_error_t error = ockam_random_interface_error_none;

  if ((random == 0) || (random->dispatch == 0)) {
    error.code = OCKAM_RANDOM_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = random->dispatch->deinit(random);

exit:
  return error;
}

ockam_error_t ockam_random_get_bytes(ockam_random_t* random, uint8_t* buffer, size_t buffer_size)
{
  ockam_error_t error = ockam_random_interface_error_none;

  if ((random == 0) || (random->dispatch == 0)) {
    error.code = OCKAM_RANDOM_INTERFACE_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = random->dispatch->get_bytes(random, buffer, buffer_size);

exit:
  return error;
}
