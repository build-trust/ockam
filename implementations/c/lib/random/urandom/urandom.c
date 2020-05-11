/**
 * @file    urandom.c
 * @brief   impl of Ockam's random functions using urandom calls
 */

#include <sys/types.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>

#include "ockam/error.h"
#include "ockam/random.h"

#include "random/impl.h"
#include "random/urandom/urandom.h"

ockam_error_t random_urandom_deinit(ockam_random_t* random);
ockam_error_t random_urandom_get_bytes(ockam_random_t* random, uint8_t* buffer, size_t buffer_size);

ockam_random_dispatch_table_t random_urandom_dispatch_table = { &random_urandom_deinit, &random_urandom_get_bytes };

ockam_error_t ockam_random_urandom_init(ockam_random_t* random)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if (random == 0) {
    error = OCKAM_RANDOM_ERROR_INVALID_PARAM;
    goto exit;
  }

  random->dispatch = &random_urandom_dispatch_table;
  random->context  = 0;

exit:
  return error;
}

ockam_error_t random_urandom_deinit(ockam_random_t* random)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

exit:
  return error;
}

ockam_error_t random_urandom_get_bytes(ockam_random_t* random, uint8_t* buffer, size_t buffer_size)
{
  ockam_error_t error         = OCKAM_ERROR_NONE;
  int           f             = 0;
  size_t        bytes_written = 0;

  if ((random == 0) || (buffer == 0)) {
    error = OCKAM_RANDOM_ERROR_INVALID_PARAM;
    goto exit;
  }

  if (buffer_size == 0) {
    error = OCKAM_RANDOM_ERROR_INVALID_SIZE;
    goto exit;
  }

  f = open("/dev/urandom", O_RDONLY);

  if (f < 0) {
    error = OCKAM_RANDOM_ERROR_GET_BYTES_FAIL;
    goto exit;
  }

  while (bytes_written < buffer_size) {
    ssize_t len = 0;

    len = read(f, (buffer + bytes_written), (buffer_size - bytes_written));

    if (len < 0) {
      if (errno == EINTR) {
        continue;
      } else {
        error = OCKAM_RANDOM_ERROR_GET_BYTES_FAIL;
        goto exit;
      }
    }

    bytes_written += (size_t) len;
  }

exit:

  if (f != -1) { close(f); }

  return error;
}
