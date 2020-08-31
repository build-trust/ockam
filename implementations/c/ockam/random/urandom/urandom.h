/**
 * @file  urandom.h
 * @brief
 */

#ifndef OCKAM_RANDOM_URANDOM_H_
#define OCKAM_RANDOM_URANDOM_H_

#include "ockam/error.h"
#include "ockam/random.h"

#include "ockam/random/impl.h"

extern const char* const OCKAM_RANDOM_URANDOM_ERROR_DOMAIN;

typedef enum {
  OCKAM_RANDOM_URANDOM_ERROR_INVALID_PARAM  = 1,
  OCKAM_RANDOM_URANDOM_ERROR_INVALID_SIZE   = 2,
  OCKAM_RANDOM_URANDOM_ERROR_GET_BYTES_FAIL = 3,
} ockam_error_code_random_urandom_t;

/**
 * @brief   Initialize the urandom random object
 * @param   random[in]  The ockam random object to initialize.
 * @return  OCKAM_ERROR_NONE on success.
 * @return  OCKAM_RANDOM_ERROR_INVALID_PARAM if invalid random pointer is received.
 */
ockam_error_t ockam_random_urandom_init(ockam_random_t* random);

#endif
