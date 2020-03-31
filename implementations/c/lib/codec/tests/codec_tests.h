#include <setjmp.h>
#include <stdarg.h>

int _test_codec_variable_length_encoded_u2le_setup(void **state);
void _test_codec_variable_length_encoded_u2le(void **state);
int _test_codec_variable_length_encoded_u2le_teardown(void **state);

int _test_codec_payload_aead_aes_gcm_setup(void **state);
void _test_codec_payload_aead_aes_gcm(void **state);
int _test_codec_payload_aead_aes_gcm_teardown(void **state);
