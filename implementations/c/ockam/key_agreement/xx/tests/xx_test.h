#include <stdlib.h>

#include "ockam/key_agreement.h"

#define ACK_TEXT "ACK_TEXT"
#define ACK_SIZE 3
#define OK       "OK"
#define OK_SIZE  2

#define TEST_MSG_BYTE_SIZE   15
#define TEST_MSG_INITIATOR   "7375626d6172696e6579656c6c6f77"
#define TEST_MSG_RESPONDER   "79656c6c6f777375626d6172696e65"
#define TEST_MSG_CIPHER_SIZE 64

#define INITIATOR_STATIC "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
#define RESPONDER_STATIC "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20"
#define INITIATOR_EPH    "202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f"
#define RESPONDER_EPH    "4142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f60"

#define MSG_1_CIPHERTEXT "358072d6365880d1aeea329adf9121383851ed21a28e3b75e965d0d2cd166254"
#define MSG_2_CIPHERTEXT                                                       \
  "64b101b1d0be5a8704bd078f9895001fc03e8e9f9522f188dd128d9846d484665393019dbd" \
  "6"                                                                          \
  "f438795da206db0886610b26108e424142c2e9b5fd1f7ea70cde8767ce62d7e3c0e9bcefe4" \
  "ab"                                                                         \
  "872c0505b9e824df091b74ffe10a2b32809cab21f"
#define MSG_3_CIPHERTEXT                                                       \
  "e610eadc4b00c17708bf223f29a66f02342fbedf6c0044736544b9271821ae40e70144cecd" \
  "9d265dffdc5bb8e051c3f83db32a425e04d8f510c58a43325fbc56"
#define MSG_4_CIPHERTEXT "9ea1da1ec3bfecfffab213e537ed170ed50de782953cb27b4c5c3339c54eca"
#define MSG_5_CIPHERTEXT "217c5111fad7afde33bd28abaff3decc280d054cdfd4784fc51d103a82ff22"

ockam_error_t xx_test_initiator(ockam_vault_t*, ockam_memory_t*, ockam_ip_address_t*, ockam_ip_address_t*);
ockam_error_t xx_test_responder(ockam_vault_t*, ockam_memory_t*, ockam_ip_address_t*);
