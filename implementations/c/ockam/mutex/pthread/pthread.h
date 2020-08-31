/**
 * @file  pthread.h
 * @brief
 */

#ifndef OCKAM_MUTEX_PTHREAD_H_
#define OCKAM_MUTEX_PTHREAD_H_

#include "ockam/error.h"
#include "ockam/mutex.h"
#include "ockam/memory.h"

#include "ockam/mutex/impl.h"

extern const char* const OCKAM_MUTEX_PTHREAD_ERROR_DOMAIN;

typedef enum {
  OCKAM_MUTEX_PTHREAD_ERROR_INVALID_PARAM = 1,
  OCKAM_MUTEX_PTHREAD_ERROR_INVALID_CONTEXT = 2,
  OCKAM_MUTEX_PTHREAD_ERROR_CREATE_FAIL = 3,
} ockam_error_code_mutex_pthread_t;

typedef struct {
  ockam_memory_t* memory;
} ockam_mutex_pthread_attributes_t;

/**
 * @brief   Initialize the pthread mutex object
 * @param   pthread[in]     The ockam mutex object to initialize.
 * @param   attributes[in]  The attributes to set for the pthread mutex.
 * @return  OCKAM_ERROR_NONE on success.
 */
ockam_error_t ockam_mutex_pthread_init(ockam_mutex_t* mutex, ockam_mutex_pthread_attributes_t* attributes);

#endif
