#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include "ockam/queue.h"
#include "ockam/syslog.h"
#include "ockam/memory.h"
#include "ockam/memory/stdlib.h"

int main()
{
  char                     nodes[8][2] = { "1", "2", "3", "4", "5", "6", "7", "8" };
  ockam_queue_t*           p_q         = NULL;
  ockam_error_t            error       = OCKAM_ERROR_NONE;
  ockam_queue_attributes_t attributes  = { 0 };
  ockam_memory_t           memory      = { 0 };
  void*                    p_node      = NULL;
  int                      ret_error   = -1;

  // Initialize
  error = ockam_memory_stdlib_init(&memory);
  if (error) goto exit;

  attributes.p_memory   = &memory;
  attributes.p_alert    = NULL;
  attributes.queue_size = 5;

  error = init_queue(&p_q, &attributes);
  if (error) goto exit;

  // Try to dequeue from an empty queue
  error = dequeue(p_q, &p_node);
  if (QUEUE_ERROR_EMPTY != error) goto exit;

  // Add one and take it back out
  error = enqueue(p_q, (void*) &nodes[0][0]);
  if (error) goto exit;

  error = dequeue(p_q, &p_node);
  if (error) goto exit;

  if (0 != strcmp((char*) p_node, &nodes[0][0])) {
    log_error(0, "Dequeue returned garbage");
    goto exit;
  }

  // Verify queue is empty
  error = dequeue(p_q, &p_node);
  if (QUEUE_ERROR_EMPTY != error) {
    log_error(0, "Dequeue on empty queue failed");
    goto exit;
  }

  // Fill up queue, then try to add when queue full
  for (int i = 0; i < 5; ++i) {
    error = enqueue(p_q, &nodes[i][0]);
    if (OCKAM_ERROR_NONE != error) {
      log_error(0, "enqueue failed while populating queue");
      goto exit;
    }
  }
  error = enqueue(p_q, (void*) "another ");
  if (QUEUE_ERROR_FULL != error) {
    log_error(0, "enqueue didn't return queue full");
    goto exit;
  }

  // Empty half-way, then refill (wrap condition)
  for (int i = 0; i < 3; ++i) {
    error = dequeue(p_q, &p_node);
    if (OCKAM_ERROR_NONE != error) {
      log_error(0, "error dequeueing while emptying half-way");
      goto exit;
    }
    if (p_node != &nodes[i][0]) {
      log_error(0, "dequeue returned wrong node");
      goto exit;
    }
  }

  // Now top of the queue, and then dequeue them all
  for (int i = 5; i < 8; ++i) {
    error = enqueue(p_q, (void*) &nodes[i]);
    if (OCKAM_ERROR_NONE != error) {
      log_error(error, "error refilling queue");
      goto exit;
    }
  }

  // Empty out entirely
  for (int i = 3; i < 8; ++i) {
    error = dequeue(p_q, &p_node);
    if (OCKAM_ERROR_NONE != error) {
      log_error(error, "error emptying queue");
      goto exit;
    }
    if (p_node != &nodes[i][0]) {
      log_error(0, "wrong node returned");
      goto exit;
    }
  }

  uint16_t p_size = 0;
  error = queue_size(p_q, &p_size);
  if (error) { goto exit; }
  if (p_size != 0) {
    log_error(error, "queue_size returned incorrect size");
    goto exit;
  }

  // Fulfill queue
  for (int i = 0; i < 5; ++i) {
    error = enqueue(p_q, nodes[i]);
    if (error) {
      log_error(error, "error fulfilling queue");
      goto exit;
    }
  }

  // Check queue size
  p_size = 0;
  error = queue_size(p_q, &p_size);
  if (error) { goto exit; }
  if (p_size != 5) {
    log_error(error, "queue_size returned incorrect size");
    goto exit;
  }

  // Check queue max size
  p_size = 0;
  error = queue_max_size(p_q, &p_size);
  if (error) { goto exit; }
  if (p_size != 5) {
    log_error(error, "queue_max_size returned incorrect max size");
    goto exit;
  }

  // Grow queue size
  error = grow_queue(p_q, 7);
  if (error) {
    log_error(error, "error growing queue");
    goto exit;
  }

  // Check queue size
  p_size = 0;
  error = queue_size(p_q, &p_size);
  if (error) goto exit;
  if (p_size != 5) {
    log_error(error, "queue_size returned incorrect size");
    goto exit;
  }

  // Check queue max size
  p_size = 0;
  error = queue_max_size(p_q, &p_size);
  if (error) goto exit;
  if (p_size != 7) {
    log_error(error, "queue_max_size returned incorrect max size");
    goto exit;
  }

  // Add more elements
  for (int i = 5; i < 7; ++i) {
    error = enqueue(p_q, nodes[i]);
    if (error) {
      log_error(error, "error queueing to grown queue");
      goto exit;
    }
  }

  // Check queue size
  p_size = 0;
  error = queue_size(p_q, &p_size);
  if (error) goto exit;
  if (p_size != 7) {
    log_error(error, "queue_size returned incorrect size");
    goto exit;
  }

  // Check queue max size
  p_size = 0;
  error = queue_max_size(p_q, &p_size);
  if (error) goto exit;
  if (p_size != 7) {
    log_error(error, "queue_max_size returned incorrect max size");
    goto exit;
  }

  // Check queue is full
  error = enqueue(p_q, nodes[7]);
  if (QUEUE_ERROR_FULL != error) {
    log_error(0, "enqueue didn't return queue full");
    goto exit;
  }

  // Grow queue even more
  error = grow_queue(p_q, 8);
  if (error) {
    log_error(error, "error growing queue");
    goto exit;
  }

  // Check queue is full
  error = enqueue(p_q, nodes[7]);
  if (error) {
    log_error(error, "error queueing to grown queue");
    goto exit;
  }

  // Check that elements are correct
  for (int i = 0; i < 8; ++i) {
    error = dequeue(p_q, &p_node);
    if (error) {
      log_error(error, "error emptying queue");
      goto exit;
    }
    if (p_node != nodes[i]) {
      log_error(0, "wrong node returned");
      goto exit;
    }
  }

  // Deinit queue
  error = uninit_queue(p_q);
  if (error) goto exit;

  // Check grow queue when head < tail
  attributes.queue_size = 2;

  error = init_queue(&p_q, &attributes);
  if (error) goto exit;

  for (int i = 0; i < 2; ++i) {
    error = enqueue(p_q, nodes[i]);
    if (error) {
      log_error(error, "error emptying queue");
      goto exit;
    }
  }

  error = grow_queue(p_q, 3);
  if (error) goto exit;

  for (int i = 0; i < 1; ++i) {
    error = dequeue(p_q, &p_node);
    if (error) {
      log_error(error, "error emptying queue");
      goto exit;
    }
    if (p_node != nodes[i]) {
      log_error(0, "wrong node returned");
      goto exit;
    }
  }

  error = uninit_queue(p_q);
  if (error) goto exit;

  ret_error = 0;
  printf("Queue test successful! (4 errors above are expected)\n");

exit:
  if (error) log_error(error, __func__);
  return ret_error;
}
