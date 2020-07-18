#ifndef KEY_AGREEMENT_IMPL_H
#define KEY_AGREEMENT_IMPL_H

#include "ockam/key_agreement.h"

#define KEY_SIZE    32
#define SHA256_SIZE 32

typedef struct ockam_key_dispatch_table_t {
  ockam_error_t (*initiate)(void*);
  ockam_error_t (*respond)(void*);
  ockam_error_t (*encrypt)(void*, uint8_t*, size_t, uint8_t*, size_t, size_t*);
  ockam_error_t (*decrypt)(void*, uint8_t*, size_t, uint8_t*, size_t, size_t*);
  ockam_error_t (*deinit)(void*);
} ockam_key_dispatch_table_t;

struct ockam_key {
  ockam_key_dispatch_table_t* dispatch;
  void*                       context;
};

#endif
