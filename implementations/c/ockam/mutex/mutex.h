/**
 * @file    mutex.h
 * @brief   Generic mutex functions for the Ockam Library
 */

#ifndef OCKAM_MUTEX_H_
#define OCKAM_MUTEX_H_

extern const char* const OCKAM_MUTEX_INTERFACE_ERROR_DOMAIN;

typedef enum {
  OCKAM_MUTEX_INTERFACE_ERROR_INVALID_PARAM = 1,
} ockam_error_code_mutex_interface_t;

/*
 * @defgroup    OCKAM_MUTEX OCKAM_MUTEX_API
 * @ingroup     OCKAM
 * @brief       OCKAM_MUTEX_API
 * @addtogroup  OCKAM_MUTEX
 * @{
 */

#include "ockam/error.h"

#include <stddef.h>
#include <stdint.h>

#define OCKAM_MUTEX_ERROR_INVALID_PARAM   (OCKAM_ERROR_INTERFACE_MUTEX | 1u)
#define OCKAM_MUTEX_ERROR_INVALID_SIZE    (OCKAM_ERROR_INTERFACE_MUTEX | 2u)
#define OCKAM_MUTEX_ERROR_INVALID_CONTEXT (OCKAM_ERROR_INTERFACE_MUTEX | 3u)
#define OCKAM_MUTEX_ERROR_CREATE_FAIL     (OCKAM_ERROR_INTERFACE_MUTEX | 4u)

struct ockam_mutex_t;
typedef struct ockam_mutex_t ockam_mutex_t;

typedef void* ockam_mutex_lock_t;

/**
 * @brief   Deinitialize the specified ockam mutex object.
 * @param   mutex[in]  The ockam mutex object to deinitialize.
 * @return  OCKAM_ERROR_NONE on success.
 */
ockam_error_t ockam_mutex_deinit(ockam_mutex_t* mutex);

/**
 * @brief   Allocate mutex from the specified mutex module
 * @param   mutex[in]
 * @param   lock[in]
 * @return  OCKAM_ERROR_NONE on success.
 */
ockam_error_t ockam_mutex_create(ockam_mutex_t* mutex, ockam_mutex_lock_t* lock);

/**
 * @brief   Destroy the specified mutex lock object.
 * @param   mutex[in]
 * @param   lock[in]
 * @return  OCKAM_ERROR_NONE on success.
 */
ockam_error_t ockam_mutex_destroy(ockam_mutex_t* mutex, ockam_mutex_lock_t lock);

/**
 * @brief   Lock the specified lock object.
 * @param   mutex[in]
 * @param   lock[in]
 * @return  OCKAM_ERROR_NONE on success.
 */
ockam_error_t ockam_mutex_lock(ockam_mutex_t* mutex, ockam_mutex_lock_t lock);

/**
 * @brief   Unlock the specified lock object.
 * @param   mutex[in]
 * @param   lock[in]
 * @return  OCKAM_ERROR_NONE on success.
 */
ockam_error_t ockam_mutex_unlock(ockam_mutex_t* mutex, ockam_mutex_lock_t lock);

/**
 * @}
 */

#endif
