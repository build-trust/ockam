#include <stdio.h>
#include <pthread.h>
#include "ockam/syslog.h"
#include "ockam/error.h"
#include "ockam/memory.h"
#include "memory/stdlib/stdlib.h"
#include "ockam/linked_list.h"
#include "ockam/queue.h"

typedef struct node {
  uint16_t     key;
  void*        data;
  struct node* prev;
  struct node* next;
} node_t;

struct ockam_linked_list {
  pthread_mutex_t list_lock;
  ockam_memory_t* p_mem;
  uint32_t        list_size;
  ockam_queue_t*  p_q;
  node_t*         p_nodes;
  node_t*         head;
};

ockam_error_t ockam_ll_init(ockam_memory_t* p_memory, size_t max_size, ockam_linked_list_t** pp_list)
{
  ockam_error_t            error  = OCKAM_ERROR_NONE;
  ockam_linked_list_t*     p_list = NULL;
  ockam_queue_attributes_t q_attrs;
  node_t*                  p_nodes = NULL;

  if (!p_memory) {
    error = LLIST_ERROR_INIT;
    goto exit;
  }

  error = ockam_memory_alloc_zeroed(p_memory, (void**) pp_list, sizeof(ockam_linked_list_t));
  if (error) goto exit;
  p_list = *pp_list;

  error = ockam_memory_alloc_zeroed(p_memory, (void**) &p_list->p_nodes, max_size * sizeof(node_t));
  if (error) goto exit;
  p_list->p_mem = p_memory;
  p_nodes       = p_list->p_nodes;

  q_attrs.p_memory   = p_memory;
  q_attrs.queue_size = max_size;
  error              = init_queue(&p_list->p_q, &q_attrs);
  if (error) goto exit;

  for (int i = 0; i < max_size; ++i) {
    error = enqueue(p_list->p_q, p_nodes);
    if (error) goto exit;
    p_nodes += sizeof(node_t);
  }

  // Create the queue lock
  if (0 != pthread_mutex_init(&p_list->list_lock, NULL)) {
    error = LLIST_ERROR_LOCK;
    goto exit;
  }

exit:
  if (error) {
    log_error(error, __func__);
    ockam_ll_uninit(p_list);
  }
  return error;
}

ockam_error_t ockam_ll_add_node(ockam_linked_list_t* p_l, uint16_t key, void* data)
{
  /**
   * Nodes are added to the tail of the list on the assumption that node lifetimes are relatively similar.
   * This has the potential pitfall that very inactive nodes will become concentrated at the front of the list,
   * slowing down search times. However, the expectation is that the queue will never be very large (a few dozen
   * entries at most) so this should not be an issue.
   *
   * Duplicate keys are allowed. Upon lookup, the first key encountered (which will be the oldest one) will be returned.
   */
  ockam_error_t error = OCKAM_ERROR_NONE;
  node_t*       node  = NULL;
  node_t*       next  = NULL;
  node_t*       prev  = NULL;

  error = dequeue(p_l->p_q, (void**) &node);
  if (error) goto exit;
  ockam_memory_set(p_l->p_mem, node, 0, sizeof(*node));
  node->key  = key;
  node->data = data;

  if (0 != pthread_mutex_lock(&p_l->list_lock)) {
    error = LLIST_ERROR_LOCK;
    goto exit;
  }

  if (p_l->head == NULL) {
    p_l->head = node;
  } else {
    node_t* n = p_l->head;
    while (n->next) { n = n->next; }
    n->next    = node;
    node->prev = n;
  }

  pthread_mutex_unlock(&p_l->list_lock);

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t ockam_ll_get_node(ockam_linked_list_t* p_l, uint16_t key, void** data)
{
  ockam_error_t error = LLIST_ERROR_NOT_FOUND;
  node_t*       node  = NULL;
  node_t*       next  = NULL;
  node_t*       prev  = NULL;

  if (0 != pthread_mutex_lock(&p_l->list_lock)) {
    error = LLIST_ERROR_LOCK;
    goto exit;
  }

  node = p_l->head;

  if (node) {
    do {
      if (node->key == key) {
        *data = node->data;
        next  = node->next;
        prev  = node->prev;
        if (next) next->prev = prev;
        if (prev) prev->next = next;
        enqueue(p_l->p_q, node);
        if (node == p_l->head) p_l->head = next;
        error = OCKAM_ERROR_NONE;
        node  = NULL;
      } else {
        node = node->next;
      }
    } while (node);
  }

  pthread_mutex_unlock(&p_l->list_lock);

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t ockam_ll_uninit(ockam_linked_list_t* p_l)
{
  if (p_l) {
    if (p_l->p_q) uninit_queue(p_l->p_q);
    if (p_l->p_nodes) ockam_memory_free(p_l->p_mem, p_l->p_nodes, 0);
  }
  if (p_l) ockam_memory_free(p_l->p_mem, p_l, 0);
  return OCKAM_ERROR_NONE;
}