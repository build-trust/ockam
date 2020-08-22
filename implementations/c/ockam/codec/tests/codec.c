#include <stdlib.h>
#include <stdint.h>
#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include "cmocka.h"
#include "codec_tests.h"

#include <stdio.h>
void print_uint8_str(uint8_t* p, uint16_t size, char* msg)
{
  printf("\n%s %d bytes: \n", msg, size);
  for (int i = 0; i < size; ++i) printf("%0.2x", *p++);
  printf("\n");
}

int main(void)
{
  const struct CMUnitTest tests[] = {
    cmocka_unit_test_setup_teardown(_test_codec_variable_length_encoded_u2le,
                                    _test_codec_variable_length_encoded_u2le_setup,
                                    _test_codec_variable_length_encoded_u2le_teardown),
    cmocka_unit_test(_test_public_key),
    cmocka_unit_test_setup_teardown(_test_local_endpoint, _test_local_endpoint_setup, _test_local_endpoint_teardown),
    cmocka_unit_test_setup_teardown(
      _test_channel_endpoint, _test_channel_endpoint_setup, _test_channel_endpoint_teardown),
    cmocka_unit_test_setup_teardown(_test_endpoints, _test_endpoints_setup, _test_endpoints_teardown),
    cmocka_unit_test(_test_route)
  };
  return cmocka_run_group_tests(tests, NULL, NULL);
}
