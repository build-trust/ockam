/**
 * @file  impl.h
 * @brief The interface for a mutex implementation
 */

#ifndef OCKAM_MUTEX_IMPL_H_
#define OCKAM_MUTEX_IMPL_H_

#include "ockam/error.h"
#include "ockam/mutex.h"

/**
 * @struct  ockam_mutex_dispatch_table
 * @brief   The Ockam Mutex implementation functions
 */
typedef struct {
  /**
   * @brief   Deinitialize the specified ockam mutex implementation.
   * @param   mutex[in]  The ockam mutex object to deinitialize.
   * @return  OCKAM_ERROR_NONE on success.
   */
  ockam_error_t (*deinit)(ockam_mutex_t* mutex);

  /**
   * @brief   Create a mutex
   * @param   mutex[in] The ockam mutex implementation to use.
   * @param   lock[out] Lock object to create.
   * @return  OCKAM_ERROR_NONE on success.
   */
  ockam_error_t (*create)(ockam_mutex_t* mutex, ockam_mutex_lock_t* lock);

  /**
   * @brief   Unlock the specified mutex
   * @param   mutex[in] The ockam mutex implementation to use.
   * @param   lock[in]  Lock object to destroy.
   * @return  OCKAM_ERROR_NONE on success.
   */
  ockam_error_t (*destroy)(ockam_mutex_t* mutex, ockam_mutex_lock_t lock);

  /**
   * @brief   Lock the specified mutex
   * @param   mutex[in] The ockam mutex implementation to use.
   * @param   lock[in]  Lock object to lock.
   * @return  OCKAM_ERROR_NONE on success.
   */
  ockam_error_t (*lock)(ockam_mutex_t* mutex, ockam_mutex_lock_t lock);

  /**
   * @brief   Unlock the specified mutex
   * @param   mutex[in] The ockam mutex implementation to use.
   * @param   lock[in]  Lock object to unlock.
   * @return  OCKAM_ERROR_NONE on success.
   */
  ockam_error_t (*unlock)(ockam_mutex_t* mutex, ockam_mutex_lock_t lock);

} ockam_mutex_dispatch_table_t;

struct ockam_mutex_t {
  ockam_mutex_dispatch_table_t* dispatch;
  void*                         context;
};

#endif
