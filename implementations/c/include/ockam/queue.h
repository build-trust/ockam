#ifndef OCKAM_QUEUE_H
#define OCKAM_QUEUE_H
#include <stdlib.h>
#include <pthread.h>
#include "ockam/error.h"
#include "ockam/memory.h"
#include "memory/stdlib/stdlib.h"

#define QUEUE_ERROR_PARAMETER  (OCKAM_ERROR_INTERFACE_QUEUE | 0x0001u)
#define QUEUE_ERROR_MUTEX      (OCKAM_ERROR_INTERFACE_QUEUE | 0x0002u)
#define QUEUE_ERROR_MUTEX_LOCK (OCKAM_ERROR_INTERFACE_QUEUE | 0x0003u)
#define QUEUE_ERROR_FULL       (OCKAM_ERROR_INTERFACE_QUEUE | 0x0004u)
#define QUEUE_ERROR_EMPTY      (OCKAM_ERROR_INTERFACE_QUEUE | 0x0005u)

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

#endif