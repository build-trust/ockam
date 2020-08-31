#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include "ockam/queue.h"
#include "ockam/log.h"
#include "ockam/memory.h"
#include "ockam/memory/stdlib.h"

int main()
{
  char                     nodes[8][2] = { "1", "2", "3", "4", "5", "6", "7", "8" };
  ockam_queue_t*           p_q         = NULL;
  ockam_queue_attributes_t attributes  = { 0 };
  ockam_memory_t           memory      = { 0 };
  void*                    p_node      = NULL;
  int                      ret_error   = -1;

  // Initialize
  ockam_error_t error = ockam_memory_stdlib_init(&memory);
  if (ockam_error_has_error(&error)) goto exit;

  attributes.p_memory   = &memory;
  attributes.p_alert    = NULL;
  attributes.queue_size = 5;

  error = init_queue(&p_q, &attributes);
  if (ockam_error_has_error(&error)) goto exit;

  // Try to dequeue from an empty queue
  error = dequeue(p_q, &p_node);
  if (OCKAM_QUEUE_ERROR_EMPTY != error.code) goto exit;

  // Add one and take it back out
  error = enqueue(p_q, (void*) &nodes[0][0]);
  if (ockam_error_has_error(&error)) goto exit;

  error = dequeue(p_q, &p_node);
  if (ockam_error_has_error(&error)) goto exit;

  if (0 != strcmp((char*) p_node, &nodes[0][0])) {
    ockam_log_error("%s", "Dequeue returned garbage");
    goto exit;
  }

  // Verify queue is empty
  error = dequeue(p_q, &p_node);
  if (OCKAM_QUEUE_ERROR_EMPTY != error.code) {
    ockam_log_error("%s", "Dequeue on empty queue failed");
    goto exit;
  }

  // Fill up queue, then try to add when queue full
  for (int i = 0; i < 5; ++i) {
    error = enqueue(p_q, &nodes[i][0]);
    if (OCKAM_ERROR_NONE != error.code) {
      ockam_log_error("%s", "enqueue failed while populating queue");
      goto exit;
    }
  }
  error = enqueue(p_q, (void*) "another ");
  if (OCKAM_QUEUE_ERROR_FULL != error.code) {
    ockam_log_error("%s", "enqueue didn't return queue full");
    goto exit;
  }

  // Empty half-way, then refill (wrap condition)
  for (int i = 0; i < 3; ++i) {
    error = dequeue(p_q, &p_node);
    if (OCKAM_ERROR_NONE != error.code) {
      ockam_log_error("%s", "error dequeueing while emptying half-way");
      goto exit;
    }
    if (p_node != &nodes[i][0]) {
      ockam_log_error("%s", "dequeue returned wrong node");
      goto exit;
    }
  }

  // Now top of the queue, and then dequeue them all
  for (int i = 5; i < 8; ++i) {
    error = enqueue(p_q, (void*) &nodes[i]);
    if (OCKAM_ERROR_NONE != error.code) {
      ockam_log_error("%s", "error refilling queue");
      goto exit;
    }
  }

  // Empty out entirely
  for (int i = 3; i < 8; ++i) {
    error = dequeue(p_q, &p_node);
    if (OCKAM_ERROR_NONE != error.code) {
      ockam_log_error("%s", "error emptying queue");
      goto exit;
    }
    if (p_node != &nodes[i][0]) {
      ockam_log_error("%s", "wrong node returned");
      goto exit;
    }
  }

  uint16_t p_size = 0;
  error = queue_size(p_q, &p_size);
  if (ockam_error_has_error(&error)) { goto exit; }
  if (p_size != 0) {
    ockam_log_error("%s", "queue_size returned incorrect size");
    goto exit;
  }

  // Fulfill queue
  for (int i = 0; i < 5; ++i) {
    error = enqueue(p_q, nodes[i]);
    if (ockam_error_has_error(&error)) {
      ockam_log_error("%s", "error fulfilling queue");
      goto exit;
    }
  }

  // Check queue size
  p_size = 0;
  error = queue_size(p_q, &p_size);
  if (ockam_error_has_error(&error)) { goto exit; }
  if (p_size != 5) {
    ockam_log_error("%s", "queue_size returned incorrect size");
    goto exit;
  }

  // Check queue max size
  p_size = 0;
  error = queue_max_size(p_q, &p_size);
  if (ockam_error_has_error(&error)) { goto exit; }
  if (p_size != 5) {
    ockam_log_error("%s", "queue_max_size returned incorrect max size");
    goto exit;
  }

  // Grow queue size
  error = grow_queue(p_q, 7);
  if (ockam_error_has_error(&error)) {
    ockam_log_error("%s", "error growing queue");
    goto exit;
  }

  // Check queue size
  p_size = 0;
  error = queue_size(p_q, &p_size);
  if (ockam_error_has_error(&error)) goto exit;
  if (p_size != 5) {
    ockam_log_error("%s", "queue_size returned incorrect size");
    goto exit;
  }

  // Check queue max size
  p_size = 0;
  error = queue_max_size(p_q, &p_size);
  if (ockam_error_has_error(&error)) goto exit;
  if (p_size != 7) {
    ockam_log_error("%s", "queue_max_size returned incorrect max size");
    goto exit;
  }

  // Add more elements
  for (int i = 5; i < 7; ++i) {
    error = enqueue(p_q, nodes[i]);
    if (ockam_error_has_error(&error)) {
      ockam_log_error("%s", "error queueing to grown queue");
      goto exit;
    }
  }

  // Check queue size
  p_size = 0;
  error = queue_size(p_q, &p_size);
  if (ockam_error_has_error(&error)) goto exit;
  if (p_size != 7) {
    ockam_log_error("%s", "queue_size returned incorrect size");
    goto exit;
  }

  // Check queue max size
  p_size = 0;
  error = queue_max_size(p_q, &p_size);
  if (ockam_error_has_error(&error)) goto exit;
  if (p_size != 7) {
    ockam_log_error("%s", "queue_max_size returned incorrect max size");
    goto exit;
  }

  // Check queue is full
  error = enqueue(p_q, nodes[7]);
  if (OCKAM_QUEUE_ERROR_FULL != error.code) {
    ockam_log_error("%s", "enqueue didn't return queue full");
    goto exit;
  }

  // Grow queue even more
  error = grow_queue(p_q, 8);
  if (ockam_error_has_error(&error)) {
    ockam_log_error("%s", "error growing queue");
    goto exit;
  }

  // Check queue is full
  error = enqueue(p_q, nodes[7]);
  if (ockam_error_has_error(&error)) {
    ockam_log_error("%s", "error queueing to grown queue");
    goto exit;
  }

  // Check that elements are correct
  for (int i = 0; i < 8; ++i) {
    error = dequeue(p_q, &p_node);
    if (ockam_error_has_error(&error)) {
      ockam_log_error("%s", "error emptying queue");
      goto exit;
    }
    if (p_node != nodes[i]) {
      ockam_log_error("%s", "wrong node returned");
      goto exit;
    }
  }

  // Deinit queue
  error = uninit_queue(p_q);
  if (ockam_error_has_error(&error)) goto exit;

  // Check grow queue when head < tail
  attributes.queue_size = 2;

  error = init_queue(&p_q, &attributes);
  if (ockam_error_has_error(&error)) goto exit;

  for (int i = 0; i < 2; ++i) {
    error = enqueue(p_q, nodes[i]);
    if (ockam_error_has_error(&error)) {
      ockam_log_error("%s", "error emptying queue");
      goto exit;
    }
  }

  error = grow_queue(p_q, 3);
  if (ockam_error_has_error(&error)) goto exit;

  for (int i = 0; i < 1; ++i) {
    error = dequeue(p_q, &p_node);
    if (ockam_error_has_error(&error)) {
      ockam_log_error("%s", "error emptying queue");
      goto exit;
    }
    if (p_node != nodes[i]) {
      ockam_log_error("%s", "wrong node returned");
      goto exit;
    }
  }

  error = uninit_queue(p_q);
  if (ockam_error_has_error(&error)) goto exit;

  ret_error = 0;
  printf("Queue test successful! (4 errors above are expected)\n");

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s", __func__);
  return ret_error;
}
