#include <setjmp.h>
#include <stdarg.h>

int  _test_codec_variable_length_encoded_u2le_setup(void** state);
void _test_codec_variable_length_encoded_u2le(void** state);
int  _test_codec_variable_length_encoded_u2le_teardown(void** state);

int  _test_codec_payload_aead_aes_gcm_setup(void** state);
void _test_codec_payload_aead_aes_gcm(void** state);
int  _test_codec_payload_aead_aes_gcm_teardown(void** state);

int  _test_codec_payload_setup(void** state);
void _test_codec_payload(void** state);
int  _test_codec_payload_teardown(void** state);

void _test_public_key(void** state);

void _test_local_endpoint(void** state);
int  _test_local_endpoint_setup(void** state);
int  _test_local_endpoint_teardown(void** state);

void _test_channel_endpoint();
int  _test_channel_endpoint_setup(void**);
int  _test_channel_endpoint_teardown(void**);

void _test_endpoints();
int  _test_endpoints_setup(void**);
int  _test_endpoints_teardown(void**);

void _test_codec_header(void** state);
void _test_route();