#ifndef KEY_AGREEMENT_IMPL_H
#define KEY_AGREEMENT_IMPL_H

#include "ockam/key_agreement.h"

#define PRIVATE_KEY_SIZE     32
#define P256_PUBLIC_KEY_SIZE 65
#define SYMMETRIC_KEY_SIZE   16
#define DH_SIZE              32
#define SHA256_SIZE          32

typedef struct ockam_key_dispatch_table_t {
  ockam_error_t (*m1_make)(void*, uint8_t*, size_t, size_t*);
  ockam_error_t (*m2_make)(void*, uint8_t*, size_t, size_t*);
  ockam_error_t (*m3_make)(void*, uint8_t*, size_t, size_t*);
  ockam_error_t (*m1_process)(void*, uint8_t*);
  ockam_error_t (*m2_process)(void*, uint8_t*);
  ockam_error_t (*m3_process)(void*, uint8_t*);
  ockam_error_t (*initiator_epilogue)(ockam_key_t* key);
  ockam_error_t (*responder_epilogue)(ockam_key_t* key);
  ockam_error_t (*encrypt)(void*, uint8_t*, size_t, uint8_t*, size_t, size_t*);
  ockam_error_t (*decrypt)(void*, uint8_t*, size_t, uint8_t*, size_t, size_t*);
  ockam_error_t (*deinit)(void*);
} ockam_key_dispatch_table_t;

struct ockam_key {
  ockam_key_dispatch_table_t* dispatch;
  void*                       context;
};

#endif
