/**
 * @file  mutex.c
 * @brief
 */

#include "ockam/error.h"
#include "ockam/mutex.h"

#include "ockam/mutex/impl.h"

ockam_error_t ockam_mutex_deinit(ockam_mutex_t* mutex)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((mutex == 0) || (mutex->dispatch == 0)) {
    error = OCKAM_MUTEX_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = mutex->dispatch->deinit(mutex);

exit:
  return error;
}

ockam_error_t ockam_mutex_create(ockam_mutex_t* mutex, ockam_mutex_lock_t *lock)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((mutex == 0) || (mutex->dispatch == 0)) {
    error = OCKAM_MUTEX_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = mutex->dispatch->create(mutex, lock);

exit:
  return error;
}

ockam_error_t ockam_mutex_destroy(ockam_mutex_t* mutex, ockam_mutex_lock_t lock)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((mutex == 0) || (mutex->dispatch == 0) || (lock == 0)) {
    error = OCKAM_MUTEX_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = mutex->dispatch->destroy(mutex, lock);

exit:
  return error;
}

ockam_error_t ockam_mutex_lock(ockam_mutex_t* mutex, ockam_mutex_lock_t lock)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((mutex == 0) || (mutex->dispatch == 0) || (lock == 0)) {
    error = OCKAM_MUTEX_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = mutex->dispatch->lock(mutex, lock);

exit:
  return error;
}

ockam_error_t ockam_mutex_unlock(ockam_mutex_t* mutex, ockam_mutex_lock_t lock)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if ((mutex == 0) || (mutex->dispatch == 0) || (lock == 0)) {
    error = OCKAM_MUTEX_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = mutex->dispatch->unlock(mutex, lock);

exit:
  return error;
}
