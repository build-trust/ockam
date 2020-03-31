#include <stdlib.h>
#include <stdint.h>
#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include "cmocka.h"
#include "codec_tests.h"

int main(void) {
  const struct CMUnitTest tests[] = {
      cmocka_unit_test_setup_teardown(_test_codec_variable_length_encoded_u2le,
                                      _test_codec_variable_length_encoded_u2le_setup,
                                      _test_codec_variable_length_encoded_u2le_teardown),
      cmocka_unit_test_setup_teardown(_test_codec_payload_aead_aes_gcm, _test_codec_payload_aead_aes_gcm_setup,
                                      _test_codec_payload_aead_aes_gcm_teardown)};
  return cmocka_run_group_tests(tests, NULL, NULL);
}
