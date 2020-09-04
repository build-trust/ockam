/**
* @file        test_atecc608a.c
* @brief
*/

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdarg.h>
#include <stddef.h>
#include <setjmp.h>

#include "ockam/error.h"
#include "ockam/vault.h"
#include "ockam/memory.h"

#include "ockam/memory/stdlib.h"
#include "ockam/vault/atecc608a.h"

#include "cryptoauthlib.h"
#include "atca_cfgs.h"
#include "atca_iface.h"
#include "atca_device.h"

#include "cmocka.h"
#include "test_vault.h"

#define PROTOCOL_NAME "AAA"
#define PROTOCOL_NAME_SIZE 3

ATCAIfaceCfg test_atecc608a_cfg =
{
  .iface_type = ATCA_I2C_IFACE,
  .devtype = ATECC608A,
  .atcai2c.slave_address = 0xC0,
  .atcai2c.bus = 1,
  .atcai2c.baud = 100000,
  .wake_delay = 1500,
  .rx_retries = 20
};

static ockam_vault_atecc608a_io_protection_t test_atecc608a_io_protection =
{                                                   /* IO Protection Key is used to encrypt data sent via */
  .key = {
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, /* I2C to the ATECC608A. During init the key is       */
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, /* written into the device. In a production system    */
    0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, /* the key should be locked into the device and never */
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37  /* transmitted via I2C.                               */
  },
  .key_size = 32,
  .slot = 6
};

typedef struct {
  ockam_vault_t  vault;
  ockam_memory_t memory;
} test_state_t;

static int test_setup(void **state)
{
  static test_state_t test_state;

  ockam_vault_atecc608a_attributes_t vault_attributes =
    {
      .memory         = &test_state.memory,
      .mutex          = 0,
      .atca_iface_cfg = &test_atecc608a_cfg,
      .io_protection  = &test_atecc608a_io_protection
    };

  ockam_error_t error = ockam_memory_stdlib_init(&test_state.memory);
  if (ockam_error_has_error(&error)) { goto exit; }

  error = ockam_vault_atecc608a_init(&test_state.vault, &vault_attributes);
  if (ockam_error_has_error(&error)) {
    printf("FAIL: Vault\r\n");
    goto exit;
  }

  *state = &test_state;

exit:
  return error.code;
}

static int test_teardown(void **state)
{
  // TODO
  return 0;
}

static void test(void **state)
{
  test_state_t* test_state = *state;
  ockam_vault_t* vault = &test_state->vault;

  assert_non_null(vault);

  ockam_vault_secret_t private_key1, private_key2;
  ockam_vault_secret_attributes_t attributes_private_key = {
    .length = OCKAM_VAULT_P256_PRIVATEKEY_LENGTH,
    .type = OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY,
    .persistence = OCKAM_VAULT_SECRET_EPHEMERAL,
    .purpose = OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT,
  };

  ockam_error_t error = ockam_vault_secret_generate(vault, &private_key1, &attributes_private_key);
  assert_int_equal(error.code, OCKAM_ERROR_NONE);

  error = ockam_vault_secret_generate(vault, &private_key2, &attributes_private_key);
  assert_int_equal(error.code, OCKAM_ERROR_NONE);

  uint8_t public_key2[OCKAM_VAULT_P256_PUBLICKEY_LENGTH];
  size_t len = 0;

  error = ockam_vault_secret_publickey_get(vault, &private_key2, public_key2, sizeof(public_key2), &len);
  assert_int_equal(error.code, OCKAM_ERROR_NONE);
  assert_int_equal(len, OCKAM_VAULT_P256_PUBLICKEY_LENGTH);

  ockam_vault_secret_t shared_secret = { 0 };

  error = ockam_vault_ecdh(vault, &private_key1, public_key2, sizeof(public_key2), &shared_secret);
  assert_int_equal(error.code, OCKAM_ERROR_NONE);

  uint8_t ck[32]; // FIXME
  ockam_memory_set(&test_state->memory, ck, 0, 32);
  ockam_memory_copy(&test_state->memory, ck, PROTOCOL_NAME, PROTOCOL_NAME_SIZE);

  ockam_vault_secret_t ck_secret;
  ockam_vault_secret_attributes_t attributes_ck = {
    .length = 32,
    .type = OCKAM_VAULT_SECRET_TYPE_CHAIN_KEY,
    .persistence = OCKAM_VAULT_SECRET_EPHEMERAL,
    .purpose = OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT,
  };

  error = ockam_vault_secret_import(vault, &ck_secret, &attributes_ck, ck, 32);
  assert_int_equal(error.code, OCKAM_ERROR_NONE);

  ockam_vault_secret_t secrets[2] = {0};
  ockam_memory_set(&test_state->memory, secrets, 0, sizeof(secrets));

  secrets[0].attributes = attributes_ck;

  ockam_vault_secret_attributes_t attributes_aes = {
    .length = 16,
    .type = OCKAM_VAULT_SECRET_TYPE_AES128_KEY,
    .persistence = OCKAM_VAULT_SECRET_EPHEMERAL,
    .purpose = OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT,
  };

  secrets[1].attributes = attributes_aes;

  error = ockam_vault_hkdf_sha256(vault, &ck_secret, &shared_secret, 2, secrets);
  assert_int_equal(error.code, OCKAM_ERROR_NONE);
}

int main(void)
{
  const struct CMUnitTest tests[] = {
    cmocka_unit_test(test),
  };

  return cmocka_run_group_tests(tests, test_setup, test_teardown);
}

