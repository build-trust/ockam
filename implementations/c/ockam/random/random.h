/**
 * @file    random.h
 * @brief   Generic random functions for the Ockam Library
 */

#ifndef OCKAM_RANDOM_H_
#define OCKAM_RANDOM_H_

/*
 * @defgroup    OCKAM_RANDOM OCKAM_RANDOM_API
 * @ingroup     OCKAM
 * @brief       OCKAM_RANDOM_API
 * @addtogroup  OCKAM_RANDOM
 * @{
 */

#include "ockam/error.h"

#include <stddef.h>
#include <stdint.h>

extern const char* const OCKAM_RANDOM_INTERFACE_ERROR_DOMAIN;

typedef enum {
  OCKAM_RANDOM_INTERFACE_ERROR_INVALID_PARAM = 1,
} ockam_error_code_random_interface_t;

struct ockam_random_t;
typedef struct ockam_random_t ockam_random_t;

/**
 * @brief   Deinitialize the specified ockam random object.
 * @param   random[in]  The ockam random object to deinitialize.
 * @return  OCKAM_ERROR_NONE on success.
 * @return  OCKAM_RANDOM_ERROR_INVALID_PARAM if invalid random received.
 */
ockam_error_t ockam_random_deinit(ockam_random_t* random);

/**
 * @brief   Generate random bytes from the specified random module
 * @param   random[in]      The ockam random object to use.
 * @param   buffer[in]      Buffer to place the random bytes in.
 * @param   buffer_size[in] Buffer size (in bytes).
 * @return  OCKAM_ERROR_NONE on success.
 * @return  OCKAM_RANDOM_ERROR_INVALID_PARAM if invalid random or buffer received.
 * @return  OCKAM_RANDOM_ERROR_INVALID_SIZE if buffer_size <=0.
 * @return  OCKAM_RANDOM_ERROR_GET_BYTES_FAIL if unable to retrieve the request bytes.
 */
ockam_error_t ockam_random_get_bytes(ockam_random_t* random, uint8_t* buffer, size_t buffer_size);

/**
 * @}
 */

#endif
