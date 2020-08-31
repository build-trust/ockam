#ifndef OCKAM_QUEUE_H
#define OCKAM_QUEUE_H
#include <stdlib.h>
#include <pthread.h>
#include "ockam/error.h"
#include "ockam/memory.h"
#include "ockam/memory/stdlib.h"

extern const char* const OCKAM_QUEUE_ERROR_DOMAIN;

typedef enum {
  OCKAM_QUEUE_ERROR_PARAMETER  = 1,
  OCKAM_QUEUE_ERROR_MUTEX      = 2,
  OCKAM_QUEUE_ERROR_MUTEX_LOCK = 3,
  OCKAM_QUEUE_ERROR_FULL       = 4,
  OCKAM_QUEUE_ERROR_EMPTY      = 5,
} ockam_error_code_queue_t;

typedef struct ockam_queue_t ockam_queue_t;

typedef struct ockam_queue_attributes_t {
  ockam_memory_t* p_memory;
  size_t          queue_size;
  pthread_cond_t* p_alert;
} ockam_queue_attributes_t;

ockam_error_t init_queue(ockam_queue_t** pp_queue, ockam_queue_attributes_t* p_attributes);
ockam_error_t enqueue(ockam_queue_t* p_q, void* node);
ockam_error_t dequeue(ockam_queue_t* p_q, void** node);
ockam_error_t uninit_queue(ockam_queue_t* p_q);
ockam_error_t queue_size(ockam_queue_t* p_q, uint16_t* p_size);
ockam_error_t queue_max_size(ockam_queue_t* p_q, uint16_t* p_size);
ockam_error_t grow_queue(ockam_queue_t* p_q, uint16_t new_max_size);

#endif
