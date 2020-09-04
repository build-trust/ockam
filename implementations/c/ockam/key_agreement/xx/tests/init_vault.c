#include "init_vault.h"

#include <ockam/vault/default.h>
#include <ockam/key_agreement.h>

#if OCKAM_ENABLE_ATECC608A_TESTS

#include "ockam/vault/atecc608a.h"
#include "cryptoauthlib.h"
#include "atca_cfgs.h"
#include "atca_iface.h"
#include "atca_device.h"

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

#endif

ockam_error_t init_vault(ockam_vault_t *vault, VAULT_OPT_t vault_opt, ockam_memory_t *memory, ockam_random_t *random) {
    ockam_error_t error = {
    OCKAM_ERROR_NONE,
    OCKAM_KEY_AGREEMENT_INTERFACE_ERROR_DOMAIN
  };

    switch (vault_opt) {
        case VAULT_OPT_DEFAULT: {
            ockam_vault_default_attributes_t vault_attributes_default = {.memory = memory, .random = random};
            error = ockam_vault_default_init(vault, &vault_attributes_default);
            break;
        }
        case VAULT_OPT_ATECC608A: {
#if OCKAM_ENABLE_ATECC608A_TESTS
            ockam_vault_atecc608a_attributes_t vault_attributes_atecc608a =
                    {
                            .memory         = memory,
                            .mutex          = 0,
                            .atca_iface_cfg = &test_atecc608a_cfg,
                            .io_protection  = &test_atecc608a_io_protection
                    };
            error = ockam_vault_atecc608a_init(vault, &vault_attributes_atecc608a);
#else
            error.code = -1;
#endif
            break;
        }
        default:
          error.code = -1;
    }

    return error;
}