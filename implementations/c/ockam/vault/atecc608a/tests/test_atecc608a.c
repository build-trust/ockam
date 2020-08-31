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

int main(void)
{
  int                                rc               = 0;
  ockam_error_t                      error            = OCKAM_ERROR_NONE;
  ockam_vault_t                      vault            = { 0 };
  ockam_memory_t                     memory           = { 0 };
  ockam_vault_atecc608a_attributes_t vault_attributes =
  {
    .memory         = &memory,
    .mutex          = 0,
    .atca_iface_cfg = &test_atecc608a_cfg,
    .io_protection  = &test_atecc608a_io_protection
  };

  cmocka_set_message_output(CM_OUTPUT_XML);

  error = ockam_memory_stdlib_init(&memory);
  if (ockam_error_has_error(&error)) {
    printf("FAIL: Memory\r\n");
    goto exit;
  }

  error = ockam_vault_atecc608a_init(&vault, &vault_attributes);
  if (ockam_error_has_error(&error)) {
    printf("FAIL: Vault\r\n");
    goto exit;
  }

  test_vault_run_random(&vault, &memory);
  test_vault_run_sha256(&vault, &memory);
  test_vault_run_secret_ecdh(&vault, &memory, OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY, 0);
  test_vault_run_hkdf(&vault, &memory);
  test_vault_run_aead_aes_gcm(&vault, &memory, TEST_VAULT_AEAD_AES_GCM_KEY_128_ONLY);

exit:
  if (ockam_error_has_error(&error)) { rc = -1; }

  return rc;
}

