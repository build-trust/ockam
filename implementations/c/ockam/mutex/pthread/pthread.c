/**
 * @file    pthread.c
 * @brief   Implementation of Ockam's mutex functions using pthread calls
 */

#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <pthread.h>

#include "ockam/error.h"
#include "ockam/mutex.h"
#include "ockam/memory.h"

#include "ockam/mutex/pthread.h"

const char* const OCKAM_MUTEX_PTHREAD_ERROR_DOMAIN = "OCKAM_MUTEX_PTHREAD_ERROR_DOMAIN";

static const ockam_error_t ockam_mutex_pthread_error_none = {
  OCKAM_ERROR_NONE,
  OCKAM_MUTEX_PTHREAD_ERROR_DOMAIN
};

typedef struct {
  ockam_memory_t *memory;
} mutex_pthread_context_t;

ockam_error_t mutex_pthread_deinit(ockam_mutex_t* mutex);
ockam_error_t mutex_pthread_create(ockam_mutex_t* mutex, ockam_mutex_lock_t *lock);
ockam_error_t mutex_pthread_destroy(ockam_mutex_t* mutex, ockam_mutex_lock_t lock);
ockam_error_t mutex_pthread_lock(ockam_mutex_t* mutex, ockam_mutex_lock_t lock);
ockam_error_t mutex_pthread_unlock(ockam_mutex_t* mutex, ockam_mutex_lock_t lock);

ockam_mutex_dispatch_table_t mutex_pthread_dispatch_table =
{
  &mutex_pthread_deinit,
  &mutex_pthread_create,
  &mutex_pthread_destroy,
  &mutex_pthread_lock,
  &mutex_pthread_unlock
};

ockam_error_t ockam_mutex_pthread_init(ockam_mutex_t* mutex, ockam_mutex_pthread_attributes_t* attributes)
{
  ockam_error_t error = ockam_mutex_pthread_error_none;
  mutex_pthread_context_t* context = 0;

  if ((mutex == 0) || (attributes == 0) || (attributes->memory == 0)) {
    error.code = OCKAM_MUTEX_PTHREAD_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  error = ockam_memory_alloc_zeroed(attributes->memory, (void**) &context, sizeof(mutex_pthread_context_t));
  if(ockam_error_is_none(&error)) {
    goto exit;
  }

  context->memory = attributes->memory;


  mutex->dispatch = &mutex_pthread_dispatch_table;
  mutex->context  = context;

exit:
  return error;
}

ockam_error_t mutex_pthread_deinit(ockam_mutex_t* mutex)
{
  ockam_error_t            error   = ockam_mutex_pthread_error_none;
  mutex_pthread_context_t* context = 0;

  if ((mutex == 0) || (mutex->context == 0)) {
    error.code = OCKAM_MUTEX_PTHREAD_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  context = (mutex_pthread_context_t*) mutex->context;

  if(context->memory == 0) {
    error.code = OCKAM_MUTEX_PTHREAD_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  error = ockam_memory_free(context->memory, mutex->context, sizeof(mutex_pthread_context_t));

  mutex->dispatch = 0;
  mutex->context  = 0;

exit:
  return error;
}

ockam_error_t mutex_pthread_create(ockam_mutex_t* mutex, ockam_mutex_lock_t* lock)
{
  ockam_error_t error = ockam_mutex_pthread_error_none;
  mutex_pthread_context_t* context = 0;

  if ((mutex == 0) || (mutex->context == 0)) {
    error.code = OCKAM_MUTEX_PTHREAD_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  context = (mutex_pthread_context_t*) mutex->context;

  if(context->memory == 0) {
    error.code = OCKAM_MUTEX_PTHREAD_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  error = ockam_memory_alloc_zeroed(context->memory, lock, sizeof(pthread_mutex_t));
  if(ockam_error_is_none(&error)) {
    goto exit;
  }

  if (pthread_mutex_init(*lock, NULL) != 0) {
    error.code = OCKAM_MUTEX_PTHREAD_ERROR_CREATE_FAIL;
    goto exit;
  }

exit:
  return error;
}

ockam_error_t mutex_pthread_destroy(ockam_mutex_t* mutex, ockam_mutex_lock_t lock)
{
  ockam_error_t error = ockam_mutex_pthread_error_none;
  mutex_pthread_context_t* context = 0;

  if ((mutex == 0) || (mutex->context == 0) || (lock == 0)) {
    error.code = OCKAM_MUTEX_PTHREAD_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  context = (mutex_pthread_context_t*) mutex->context;

  if(context->memory == 0) {
    error.code = OCKAM_MUTEX_PTHREAD_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  pthread_mutex_destroy(lock);

  error = ockam_memory_free(context->memory, lock, sizeof(pthread_mutex_t));
  if(ockam_error_is_none(&error)) {
    goto exit;
  }

exit:
  return error;
}

ockam_error_t mutex_pthread_lock(ockam_mutex_t* mutex, ockam_mutex_lock_t lock)
{
  ockam_error_t error = ockam_mutex_pthread_error_none;
  mutex_pthread_context_t* context = 0;
  pthread_mutex_t* pthread_lock;

  if ((mutex == 0) || (mutex->context == 0) || (lock == 0)) {
    error.code = OCKAM_MUTEX_PTHREAD_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  pthread_mutex_lock(lock);

exit:
  return error;
}

ockam_error_t mutex_pthread_unlock(ockam_mutex_t* mutex, ockam_mutex_lock_t lock)
{
  ockam_error_t error = ockam_mutex_pthread_error_none;
  mutex_pthread_context_t* context = 0;
  pthread_mutex_t* pthread_lock;

  if ((mutex == 0) || (mutex->context == 0) || (lock == 0)) {
    error.code = OCKAM_MUTEX_PTHREAD_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  pthread_mutex_unlock(lock);

exit:
  return error;
}
