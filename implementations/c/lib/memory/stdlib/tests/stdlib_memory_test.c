#include <stdarg.h>
#include <stddef.h>
#include <setjmp.h>
#include <cmocka.h>
#include <stdio.h>
#include "ockam/memory.h"
#include "memory/stdlib/stdlib.h"

static void memory_compare__null_memory__should_return_error(void **state)
{
  (void) state;

  ockam_memory_t memory = { 0 };
  ockam_error_t error   = ockam_memory_stdlib_init(&memory);
  assert_int_equal(error, OCKAM_ERROR_NONE);

  char block1[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };
  char block2[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };

  int res = 2;
  error   = ockam_memory_compare(NULL, &res, block1, block2, sizeof(block1));
  assert_int_equal(error, OCKAM_MEMORY_ERROR_INVALID_PARAM);
}

static void memory_compare__null_res__should_return_error(void **state)
{
  (void) state;

  ockam_memory_t memory = { 0 };
  ockam_error_t error   = ockam_memory_stdlib_init(&memory);
  assert_int_equal(error, OCKAM_ERROR_NONE);

  char block1[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };
  char block2[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };

  int res = 2;
  error   = ockam_memory_compare(&memory, NULL, block1, block2, sizeof(block1));
  assert_int_equal(error, OCKAM_MEMORY_ERROR_INVALID_PARAM);
}

static void memory_compare__null_lhs__should_return_error(void **state)
{
  (void) state;

  ockam_memory_t memory = { 0 };
  ockam_error_t error   = ockam_memory_stdlib_init(&memory);
  assert_int_equal(error, OCKAM_ERROR_NONE);

  char block1[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };
  char block2[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };

  int res = 2;
  error   = ockam_memory_compare(&memory, &res, NULL, block2, sizeof(block1));
  assert_int_equal(error, OCKAM_MEMORY_ERROR_INVALID_PARAM);
}

static void memory_compare__null_rhs__should_return_error(void **state)
{
  (void) state;

  ockam_memory_t memory = { 0 };
  ockam_error_t error   = ockam_memory_stdlib_init(&memory);
  assert_int_equal(error, OCKAM_ERROR_NONE);

  char block1[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };
  char block2[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };

  int res = 2;
  error   = ockam_memory_compare(&memory, &res, block1, NULL, sizeof(block1));
  assert_int_equal(error, OCKAM_MEMORY_ERROR_INVALID_PARAM);
}

static void memory_compare__empty_blocks__should_return_zero(void **state)
{
  (void) state;

  ockam_memory_t memory = { 0 };
  ockam_error_t error   = ockam_memory_stdlib_init(&memory);
  assert_int_equal(error, OCKAM_ERROR_NONE);

  char block1[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };
  char block2[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };

  int res = 2;
  error   = ockam_memory_compare(&memory, &res, block1, block2, 0);
  assert_int_equal(error, OCKAM_ERROR_NONE);
  assert_int_equal(res, 0);
}

static void memory_compare__eq_blocks__should_return_zero(void **state)
{
  (void) state;

  ockam_memory_t memory = { 0 };
  ockam_error_t error   = ockam_memory_stdlib_init(&memory);
  assert_int_equal(error, OCKAM_ERROR_NONE);

  char block1[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };
  char block2[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };

  int res = 2;
  error = ockam_memory_compare(&memory, &res, block1, block2, sizeof(block1));
  assert_int_equal(error, OCKAM_ERROR_NONE);
  assert_int_equal(res, 0);
}

static void memory_compare__lt_blocks__should_return_minus_one(void **state)
{
  (void) state;

  ockam_memory_t memory = { 0 };
  ockam_error_t error   = ockam_memory_stdlib_init(&memory);
  assert_int_equal(error, OCKAM_ERROR_NONE);

  char block1[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };
  char block2[5] = { 0x00, 0x01, 0x02, 0x03, 0x05 };

  int res = 2;

  error = ockam_memory_compare(&memory, &res, block1, block2, sizeof(block1));
  assert_int_equal(error, OCKAM_ERROR_NONE);
  assert_int_equal(res, -1);
}

static void memory_compare__gt_blocks__should_return_one(void **state)
{
  (void) state;

  ockam_memory_t memory = { 0 };
  ockam_error_t error   = ockam_memory_stdlib_init(&memory);
  assert_int_equal(error, OCKAM_ERROR_NONE);

  char block1[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };
  char block2[5] = { 0x00, 0x01, 0x02, 0x03, 0x03 };

  int res = 2;

  error = ockam_memory_compare(&memory, &res, block1, block2, sizeof(block1));
  assert_int_equal(error, OCKAM_ERROR_NONE);
  assert_int_equal(res, 1);
}

int main(void)
{
  const struct CMUnitTest tests[] = {
    cmocka_unit_test(memory_compare__null_memory__should_return_error),
    cmocka_unit_test(memory_compare__null_res__should_return_error),
    cmocka_unit_test(memory_compare__null_lhs__should_return_error),
    cmocka_unit_test(memory_compare__null_rhs__should_return_error),
    cmocka_unit_test(memory_compare__empty_blocks__should_return_zero),
    cmocka_unit_test(memory_compare__eq_blocks__should_return_zero),
    cmocka_unit_test(memory_compare__lt_blocks__should_return_minus_one),
    cmocka_unit_test(memory_compare__gt_blocks__should_return_one),
  };
  
  return cmocka_run_group_tests(tests, NULL, NULL);
}
