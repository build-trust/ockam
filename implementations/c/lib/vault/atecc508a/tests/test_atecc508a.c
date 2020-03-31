/**
********************************************************************************************************
* @file        test_atecc508a.c
* @brief
********************************************************************************************************
*/

/*
 ********************************************************************************************************
 *                                             INCLUDE FILES                                            *
 ********************************************************************************************************
 */

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>

#include <stdarg.h>
#include <stddef.h>
#include <setjmp.h>
#include <cmocka.h>

#include "ockam/error.h"
#include "ockam/vault.h"
#include "ockam/memory.h"

#include "cryptoauthlib.h"
#include "atca_cfgs.h"
#include "atca_iface.h"
#include "atca_device.h"

#include "test_vault.h"
#include "atecc508a.h"

/*
 ********************************************************************************************************
 *                                                DEFINES                                               *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                               CONSTANTS                                              *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                               DATA TYPES                                             *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES                                         *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                            GLOBAL VARIABLES                                          *
 ********************************************************************************************************
 */

ATCAIfaceCfg atca_iface_i2c = {.iface_type = ATCA_I2C_IFACE,
                               .devtype = ATECC508A,
                               {
                                   .atcai2c.slave_address = 0xB0,
                                   .atcai2c.bus = 1,
                                   .atcai2c.baud = 100000,
                               },
                               .wake_delay = 1500,
                               .rx_retries = 20};

OckamVaultAtecc508aConfig atecc508a_cfg = {.ec = kOckamVaultEcP256, .p_atca_iface_cfg = &atca_iface_i2c};

const OckamVault *vault = &ockam_vault_atecc508a;
const OckamMemory *memory = &ockam_memory_stdlib;

/*
 ********************************************************************************************************
 *                                           GLOBAL FUNCTIONS                                           *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                            LOCAL FUNCTIONS                                           *
 ********************************************************************************************************
 */

/**
 ********************************************************************************************************
 *                                             main()
 *
 * @brief   Main point of entry for mbedcrypto test
 *
 ********************************************************************************************************
 */

int main(void) {
  OckamError err;
  uint8_t i;
  void *atecc508a_0 = 0;

  memory->Create(0); /* Always initialize memory first!                    */

  cmocka_set_message_output(CM_OUTPUT_XML); /* Configure the unit test output for JUnit XML       */

  /* ---------- */
  /* Vault Init */
  /* ---------- */

  vault->Create(&atecc508a_0, /* Create a vault using ATECC508A                     */
                &atecc508a_cfg, memory);
  if (err != kOckamErrorNone) { /* Ensure it initialized before proceeding, otherwise */
    return -1;                  /* don't bother trying to run any other tests         */
  }

  /* ------------------------ */
  /* Random Number Generation */
  /* ------------------------ */

  TestVaultRunRandom(vault, atecc508a_0, memory);

  /* --------------------- */
  /* Key Generation & ECDH */
  /* --------------------- */

  TestVaultRunKeyEcdh(vault, atecc508a_0, memory, atecc508a_cfg.ec, 0);

  /* ------ */
  /* SHA256 */
  /* ------ */

  TestVaultRunSha256(vault, atecc508a_0, memory);

  /* -----*/
  /* HKDF */
  /* -----*/

  TestVaultRunHkdf(vault, atecc508a_0, memory);

  /* -------------------- */
  /* AES GCM Calculations */
  /* -------------------- */

  TestVaultRunAesGcm(vault, atecc508a_0, memory);

  return 0;
}
