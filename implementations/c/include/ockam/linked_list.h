#ifndef OCKAM_LINKED_LIST
#define OCKAM_LINKED_LIST
#include "ockam/error.h"

#define LLIST_ERROR_INIT      OCKAM_ERROR_INTERFACE_LINKED_LIST | 0x0001u
#define LLIST_ERROR_LOCK      OCKAM_ERROR_INTERFACE_LINKED_LIST | 0x0002u
#define LLIST_ERROR_NOT_FOUND OCKAM_ERROR_INTERFACE_LINKED_LIST | 0x0003u

typedef struct ockam_linked_list ockam_linked_list_t;

ockam_error_t ockam_ll_init(ockam_memory_t* p_memory, size_t max_size, ockam_linked_list_t** pp_list);
ockam_error_t ockam_ll_add_node(ockam_linked_list_t* p_l, uint16_t key, void* data);
ockam_error_t ockam_ll_get_node(ockam_linked_list_t* p_l, uint16_t key, void** data);
ockam_error_t ockam_ll_uninit(ockam_linked_list_t* p_l);

#endif