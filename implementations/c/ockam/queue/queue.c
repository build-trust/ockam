#include <stdlib.h>
#include <string.h>
#include <pthread.h>
#include "ockam/queue.h"
#include "ockam/error.h"
#include "ockam/log.h"

const char* const OCKAM_QUEUE_ERROR_DOMAIN = "OCKAM_QUEUE_ERROR_DOMAIN";

static const ockam_error_t ockam_queue_error_none = {
  OCKAM_ERROR_NONE,
  OCKAM_QUEUE_ERROR_DOMAIN
};

struct ockam_queue_t {
  ockam_memory_t* p_memory;
  uint16_t        max_size;
  uint16_t        size;
  uint16_t        head;
  uint16_t        tail;
  pthread_mutex_t modify_lock;
  pthread_cond_t* p_alert;
  void**          nodes;
};

ockam_error_t init_queue(ockam_queue_t** pp_queue, ockam_queue_attributes_t* p_attributes)
{
  ockam_error_t  error      = ockam_queue_error_none;
  ockam_queue_t* p_queue    = NULL;

  if ((NULL == p_attributes) || (NULL == pp_queue)) {
    error.code = OCKAM_QUEUE_ERROR_PARAMETER;
    goto exit;
  }
  if ((p_attributes->queue_size < 1) || (NULL == p_attributes->p_memory)) {
    error.code = OCKAM_QUEUE_ERROR_PARAMETER;
    goto exit;
  }
  *pp_queue = NULL;

  // Allocate memory for queue struct
  error             = ockam_memory_alloc_zeroed(p_attributes->p_memory, (void**) &p_queue, sizeof(ockam_queue_t));
  if (ockam_error_has_error(&error)) goto exit;
  p_queue->max_size = p_attributes->queue_size;
  p_queue->p_memory = p_attributes->p_memory;

  // Allocate memory for nodes
  size_t nodes_size = p_attributes->queue_size * sizeof(void*);
  error             = ockam_memory_alloc_zeroed(p_attributes->p_memory, (void**)&(p_queue->nodes), nodes_size);
  if (ockam_error_has_error(&error)) goto exit;

  // Create the queue lock
  if (0 != pthread_mutex_init(&p_queue->modify_lock, NULL)) {
    error.code = OCKAM_QUEUE_ERROR_MUTEX;
    goto exit;
  }

  // Save the alert condition, if one was given
  if (NULL != p_attributes->p_alert) p_queue->p_alert = p_attributes->p_alert;

  // Success
  *pp_queue = p_queue;

exit:
  if (ockam_error_has_error(&error) && (NULL != p_queue)) {
    pthread_mutex_destroy(&p_queue->modify_lock);
    ockam_memory_free(p_attributes->p_memory, p_queue->nodes, nodes_size);
    ockam_memory_free(p_attributes->p_memory, p_queue, sizeof(ockam_queue_t));
  }
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
};

ockam_error_t enqueue(ockam_queue_t* p_q, void* node)
{
  ockam_error_t error       = ockam_queue_error_none;
  int16_t       q_is_locked = 0;

  // Validate parameters
  if ((NULL == p_q) || (NULL == node)) {
    ockam_log_error("%s", "Invalid parameter in enqueue");
    error.code = OCKAM_QUEUE_ERROR_PARAMETER;
    goto exit;
  }

  // Lock the queue
  if (0 != pthread_mutex_lock(&p_q->modify_lock)) {
    error.code = OCKAM_QUEUE_ERROR_MUTEX_LOCK;
    goto exit;
  }
  q_is_locked = 1;

  // Check for queue full
  if (p_q->size == p_q->max_size) {
    // TODO: Would it be better to instead grow queue?
    error.code = OCKAM_QUEUE_ERROR_FULL;
    goto exit;
  }

  // Add node to queue tail and bump queue size
  p_q->nodes[p_q->tail] = node;
  p_q->tail             = (p_q->tail + 1) % p_q->max_size;
  p_q->size += 1;

  // Trigger the alert condition, if we have one
  if (NULL != p_q->p_alert) { pthread_cond_signal(p_q->p_alert); }

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  if (q_is_locked) pthread_mutex_unlock(&p_q->modify_lock);
  return error;
}

ockam_error_t dequeue(ockam_queue_t* p_q, void** pp_node)
{
  ockam_error_t error       = ockam_queue_error_none;
  int16_t       q_is_locked = 0;

  // Validate parameters
  if ((NULL == p_q) || (NULL == pp_node)) {
    ockam_log_error("%s", "invalid parameter in dequeue");
    error.code = OCKAM_QUEUE_ERROR_PARAMETER;
    goto exit;
  }

  // Lock the queue
  if (0 != pthread_mutex_lock(&p_q->modify_lock)) {
    error.code = OCKAM_QUEUE_ERROR_MUTEX_LOCK;
    goto exit;
  }
  q_is_locked = 1;

  // Check for queue empty
  if (0 == p_q->size) {
    error.code = OCKAM_QUEUE_ERROR_EMPTY;
    goto exit;
  }

  // Dequeue node and decrease size
  *pp_node              = p_q->nodes[p_q->head];
  p_q->nodes[p_q->head] = NULL;
  p_q->head             = (p_q->head + 1) % p_q->max_size;
  p_q->size -= 1;

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  if (q_is_locked) pthread_mutex_unlock(&p_q->modify_lock);
  return error;
}

ockam_error_t uninit_queue(ockam_queue_t* p_q)
{
  ockam_error_t   error       = ockam_queue_error_none;
  int16_t         q_is_locked = 0;
  pthread_mutex_t lock;

  // Validate parameters
  if (NULL == p_q) {
    error.code = OCKAM_QUEUE_ERROR_PARAMETER;
    goto exit;
  }

  // Lock the queue
  if (0 != pthread_mutex_lock(&p_q->modify_lock)) {
    error.code = OCKAM_QUEUE_ERROR_MUTEX_LOCK;
    goto exit;
  }
  q_is_locked = 1;
  lock        = p_q->modify_lock;

  // Free up the memory
  error = ockam_memory_free(p_q->p_memory, p_q->nodes, 0);
  if (ockam_error_has_error(&error)) {
    goto exit;
  }

  // TODO: passing 0 as len may not work properly with all implementations
  error = ockam_memory_free(p_q->p_memory, p_q, 0);

exit:
  if (q_is_locked) pthread_mutex_unlock(&lock);
  return error;
}

ockam_error_t queue_max_size(ockam_queue_t* p_q, uint16_t* p_size)
{
  ockam_error_t error = ockam_queue_error_none;

  if (NULL == p_q || NULL == p_size) {
    error.code = OCKAM_QUEUE_ERROR_PARAMETER;
    goto exit;
  }

  *p_size = p_q->max_size;

exit:
  return error;
}

ockam_error_t queue_size(ockam_queue_t* p_q, uint16_t* p_size)
{
  ockam_error_t error = ockam_queue_error_none;

  if (NULL == p_q || NULL == p_size) {
    error.code = OCKAM_QUEUE_ERROR_PARAMETER;
    goto exit;
  }

  *p_size = p_q->size;

exit:
  return error;
}

ockam_error_t grow_queue(ockam_queue_t* p_q, uint16_t new_max_size)
{
  ockam_error_t error       = ockam_queue_error_none;
  int16_t       q_is_locked = 0;

  // Validate parameters
  if ((NULL == p_q) || (new_max_size <= p_q->max_size)) {
    ockam_log_error("%s", "Invalid parameter in grow_queue");
    error.code = OCKAM_QUEUE_ERROR_PARAMETER;
    goto exit;
  }

  // Lock the queue
  if (0 != pthread_mutex_lock(&p_q->modify_lock)) {
    error.code = OCKAM_QUEUE_ERROR_MUTEX_LOCK;
    goto exit;
  }
  q_is_locked = 1;

  // Allocate memory for new nodes
  size_t nodes_size = new_max_size * sizeof(void*);
  void** new_nodes  = NULL;
  error             = ockam_memory_alloc_zeroed(p_q->p_memory, (void**)&new_nodes, nodes_size);
  if (ockam_error_has_error(&error)) goto exit;

  // Copy old nodes
  if (p_q->size != 0) {
    if (p_q->tail > p_q->head) {
      error = ockam_memory_copy(p_q->p_memory, &new_nodes[0], &p_q->nodes[p_q->head], p_q->size * sizeof(void*));
    } else {
      size_t size1 = p_q->max_size - p_q->head;
      error = ockam_memory_copy(p_q->p_memory, &new_nodes[0], &p_q->nodes[p_q->head], size1 * sizeof(void*));
      if (ockam_error_has_error(&error)) goto exit;
      ockam_memory_copy(p_q->p_memory, &new_nodes[size1], &p_q->nodes[0], (p_q->size - size1) * sizeof(void*));
    }
  }

  if (ockam_error_has_error(&error)) goto exit;

  p_q->max_size = new_max_size;
  error = ockam_memory_free(p_q->p_memory, p_q->nodes, 0);
  if (ockam_error_has_error(&error)) goto exit;
  p_q->nodes = new_nodes;
  p_q->head = 0;
  p_q->tail = (p_q->head + p_q->size) % new_max_size;

exit:
  if (q_is_locked) pthread_mutex_unlock(&p_q->modify_lock);
  if (ockam_error_has_error(&error)) {
    ockam_memory_free(p_q->p_memory, new_nodes, 0);
    ockam_log_error("%s: %d", error.domain, error.code);
  }
  return error;
}
