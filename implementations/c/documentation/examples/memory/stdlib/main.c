/**
 * @file  main.c
 * @brief
 */

#include <stdio.h>

#include "ockam/error.h"
#include "ockam/memory.h"
#include "ockam/memory/stdlib.h"

#define EXAMPLE_MEMORY_BUFFER_SIZE 16u

int main(void)
{
  ockam_error_t ret_val = OCKAM_ERROR_NONE;
  ockam_memory_t memory;
  int exit_code = 0;
  uint8_t *p_buf0 = 0;
  uint8_t *p_buf1 = 0;


  ret_val = ockam_memory_stdlib_init(&memory);
  if(ret_val != OCKAM_ERROR_NONE) {
    goto exit_block;
  }

  ret_val = ockam_memory_alloc(&memory, &p_buf0, EXAMPLE_MEMORY_BUFFER_SIZE);
  if(ret_val != OCKAM_ERROR_NONE) {
    goto exit_block;
  }

  ret_val = ockam_memory_alloc(&memory, &p_buf1, EXAMPLE_MEMORY_BUFFER_SIZE);
  if(ret_val != OCKAM_ERROR_NONE) {
    goto exit_block;
  }

  ret_val = ockam_memory_set(&memory, p_buf0, 0xA5, EXAMPLE_MEMORY_BUFFER_SIZE);
  if(ret_val != OCKAM_ERROR_NONE) {
    goto exit_block;
  }

  ret_val = ockam_memory_copy(&memory, p_buf1, p_buf0, EXAMPLE_MEMORY_BUFFER_SIZE);
  if(ret_val != OCKAM_ERROR_NONE) {
    goto exit_block;
  }

  ret_val = ockam_memory_free(&memory, p_buf0, EXAMPLE_MEMORY_BUFFER_SIZE);
  if(ret_val != OCKAM_ERROR_NONE) {
    goto exit_block;
  }

  ret_val = ockam_memory_free(&memory, p_buf1, EXAMPLE_MEMORY_BUFFER_SIZE);
  if(ret_val != OCKAM_ERROR_NONE) {
    goto exit_block;
  }

  ret_val = ockam_memory_deinit(&memory);

exit_block:
  if(ret_val != OCKAM_ERROR_NONE) {
    exit_code = -1;
  }

  return exit_code;
}

