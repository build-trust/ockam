#include <stdarg.h>
#include <stddef.h>
#include <setjmp.h>
#include <cmocka.h>
#include <stdio.h>
#include "ockam/memory.h"
#include "ockam/memory/stdlib.h"

#define MAX_MEMORY_IMPLEMENTATIONS_COUNT 5

typedef struct {
  ockam_memory_t   memory[MAX_MEMORY_IMPLEMENTATIONS_COUNT];
  size_t           implementations_count;
} test_state_t;

static int test_setup(void **state)
{
  static test_state_t test_state;

  size_t i = 0;

  // ifdef should be added here when more memory implementations added
  {
    ockam_memory_t* p_memory = &test_state.memory[i++];
    ockam_error_t      error = ockam_memory_stdlib_init(p_memory);
    assert_true(ockam_error_is_none(&error));
    assert_ptr_equal(error.domain, OCKAM_MEMORY_STDLIB_ERROR_DOMAIN);
  }

  test_state.implementations_count = i;

  *state = &test_state;

  return 0;
}

static int test_teardown(void **state)
{
  test_state_t* test_state = *state;

  for (int i = 0; i < test_state->implementations_count; i++) {
    ockam_memory_deinit(&test_state->memory[i]);
  }

  return 0;
}

static void memory_compare__null_memory__should_return_error(void **state)
{
  (void) state;

  char block1[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };
  char block2[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };

  int           res   = 2;
  ockam_error_t error = ockam_memory_compare(NULL, &res, block1, block2, sizeof(block1));
  assert_int_equal(error.code, OCKAM_MEMORY_STDLIB_ERROR_INVALID_PARAM);
  assert_ptr_equal(error.domain, OCKAM_MEMORY_INTERFACE_ERROR_DOMAIN);
}

static void memory_compare__null_dispatch__should_return_error(void **state)
{
  (void) state;

  char block1[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };
  char block2[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };

  ockam_memory_t memory = { 0 };

  int           res   = 2;
  ockam_error_t error = ockam_memory_compare(&memory, &res, block1, block2, sizeof(block1));
  assert_int_equal(error.code, OCKAM_MEMORY_STDLIB_ERROR_INVALID_PARAM);
  assert_ptr_equal(error.domain, OCKAM_MEMORY_INTERFACE_ERROR_DOMAIN);
}

static void memory_compare__null_res__should_return_error(void **state)
{
  test_state_t* test_state = *state;

  char block1[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };
  char block2[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };

  for (int i = 0; i < test_state->implementations_count; i++) {
    ockam_memory_t* p_memory = &test_state->memory[i];

    ockam_error_t error   = ockam_memory_compare(p_memory, NULL, block1, block2, sizeof(block1));
    assert_int_equal(error.code, OCKAM_MEMORY_STDLIB_ERROR_INVALID_PARAM);
    assert_ptr_equal(error.domain, OCKAM_MEMORY_STDLIB_ERROR_DOMAIN);
  }
}

static void memory_compare__null_lhs__should_return_error(void **state)
{
  test_state_t* test_state = *state;

  char block2[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };

  for (int i = 0; i < test_state->implementations_count; i++) {
    ockam_memory_t* p_memory = &test_state->memory[i];

    int           res   = 2;
    ockam_error_t error = ockam_memory_compare(p_memory, &res, NULL, block2, sizeof(block2));
    assert_int_equal(error.code, OCKAM_MEMORY_STDLIB_ERROR_INVALID_PARAM);
    assert_ptr_equal(error.domain, OCKAM_MEMORY_STDLIB_ERROR_DOMAIN);
  }
}

static void memory_compare__null_rhs__should_return_error(void **state)
{
  test_state_t* test_state = *state;

  char block1[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };

  for (int i = 0; i < test_state->implementations_count; i++) {
    ockam_memory_t* p_memory = &test_state->memory[i];

    int           res   = 2;
    ockam_error_t error = ockam_memory_compare(p_memory, &res, block1, NULL, sizeof(block1));
    assert_int_equal(error.code, OCKAM_MEMORY_STDLIB_ERROR_INVALID_PARAM);
    assert_ptr_equal(error.domain, OCKAM_MEMORY_STDLIB_ERROR_DOMAIN);
  }
}

static void memory_compare__empty_blocks__should_return_zero(void **state)
{
  test_state_t* test_state = *state;

  char block1[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };
  char block2[5] = { 0x01, 0x02, 0x03, 0x04, 0x05 };

  for (int i = 0; i < test_state->implementations_count; i++) {
    ockam_memory_t* p_memory = &test_state->memory[i];

    int           res     = 2;
    ockam_error_t error   = ockam_memory_compare(p_memory, &res, block1, block2, 0);
    assert_true(ockam_error_is_none(&error));
    assert_ptr_equal(error.domain, OCKAM_MEMORY_STDLIB_ERROR_DOMAIN);
    assert_int_equal(res, 0);
  }
}

static void memory_compare__eq_blocks__should_return_zero(void **state)
{
  test_state_t* test_state = *state;

  char block1[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };
  char block2[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };

  for (int i = 0; i < test_state->implementations_count; i++) {
    ockam_memory_t* p_memory = &test_state->memory[i];

    int           res   = 2;
    ockam_error_t error = ockam_memory_compare(p_memory, &res, block1, block2, sizeof(block1));
    assert_true(ockam_error_is_none(&error));
    assert_ptr_equal(error.domain, OCKAM_MEMORY_STDLIB_ERROR_DOMAIN);
    assert_int_equal(res, 0);
  }
}

static void memory_compare__lt_blocks__should_return_minus_one(void **state)
{
  test_state_t* test_state = *state;

  char block1[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };
  char block2[5] = { 0x00, 0x01, 0x02, 0x03, 0x05 };

  for (int i = 0; i < test_state->implementations_count; i++) {
    ockam_memory_t* p_memory = &test_state->memory[i];

    int           res   = 2;
    ockam_error_t error = ockam_memory_compare(p_memory, &res, block1, block2, sizeof(block1));
    assert_true(ockam_error_is_none(&error));
    assert_ptr_equal(error.domain, OCKAM_MEMORY_STDLIB_ERROR_DOMAIN);
    assert_int_equal(res, -1);
  }
}

static void memory_compare__gt_blocks__should_return_one(void **state)
{
  test_state_t* test_state = *state;

  char block1[5] = { 0x00, 0x01, 0x02, 0x03, 0x04 };
  char block2[5] = { 0x00, 0x01, 0x02, 0x03, 0x03 };

  for (int i = 0; i < test_state->implementations_count; i++) {
    ockam_memory_t* p_memory = &test_state->memory[i];

    int           res   = 2;
    ockam_error_t error = ockam_memory_compare(p_memory, &res, block1, block2, sizeof(block1));
    assert_true(ockam_error_is_none(&error));
    assert_ptr_equal(error.domain, OCKAM_MEMORY_STDLIB_ERROR_DOMAIN);
    assert_int_equal(res, 1);
  }
}

int main(void)
{
  const struct CMUnitTest tests[] = {
    cmocka_unit_test(memory_compare__null_memory__should_return_error),
    cmocka_unit_test(memory_compare__null_dispatch__should_return_error),
    cmocka_unit_test_setup_teardown(memory_compare__null_res__should_return_error, test_setup, test_teardown),
    cmocka_unit_test_setup_teardown(memory_compare__null_lhs__should_return_error, test_setup, test_teardown),
    cmocka_unit_test_setup_teardown(memory_compare__null_rhs__should_return_error, test_setup, test_teardown),
    cmocka_unit_test_setup_teardown(memory_compare__empty_blocks__should_return_zero, test_setup, test_teardown),
    cmocka_unit_test_setup_teardown(memory_compare__eq_blocks__should_return_zero, test_setup, test_teardown),
    cmocka_unit_test_setup_teardown(memory_compare__lt_blocks__should_return_minus_one, test_setup, test_teardown),
    cmocka_unit_test_setup_teardown(memory_compare__gt_blocks__should_return_one, test_setup, test_teardown),
  };

  return cmocka_run_group_tests(tests, NULL, NULL);
}
