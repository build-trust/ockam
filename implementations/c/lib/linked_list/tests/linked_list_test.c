#include <stdio.h>
#include "ockam/syslog.h"
#include "ockam/error.h"
#include "ockam/memory.h"
#include "memory/stdlib/stdlib.h"
#include "ockam/linked_list.h"

int main()
{
  ockam_error_t        error    = OCKAM_ERROR_NONE;
  ockam_memory_t       memory   = { 0 };
  ockam_linked_list_t* p_l      = NULL;
  uint16_t             data[20] = { 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19 };
  uint16_t*            d;

  error = ockam_memory_stdlib_init(&memory);
  if (error) goto exit;

  error = ockam_ll_init(&memory, 20, &p_l);
  if (error) goto exit;

  error = ockam_ll_add_node(p_l, 5, &data[5]);
  if (error) goto exit;

  error = ockam_ll_get_node(p_l, 5, (void*) &d);
  if (error) goto exit;
  if (*d != 5) {
    error = 1;
    goto exit;
  }

  for (int i = 0; i < 20; ++i) { error = ockam_ll_add_node(p_l, i, &data[i]); }

  for (int i = 4; i >= 0; --i) {
    error = ockam_ll_get_node(p_l, i, (void*) &d);
    if (error) goto exit;
    if (i != *d) {
      error = 1;
      goto exit;
    }
  }

  for (int i = 5; i < 20; ++i) {
    error = ockam_ll_get_node(p_l, i, (void*) &d);
    if (error) goto exit;
    if (i != *d) {
      error = 1;
      goto exit;
    }
  }

exit:
  if (error) log_error(error, __func__);
  if (p_l) ockam_ll_uninit(p_l);
  return error;
}