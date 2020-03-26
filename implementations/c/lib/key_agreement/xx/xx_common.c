#include <string.h>
#include <stdio.h>

#include "ockam/error.h"
#include "ockam/key_agreement.h"
#include "ockam/syslog.h"
#include "ockam/vault.h"
#include "xx_local.h"

/*
 ********************************************************************************************************
 *                                           GLOBAL FUNCTIONS *
 ********************************************************************************************************
 */

OckamError OckamKeyInitializeXX(KeyEstablishmentXX *xx, const OckamVault *vault, OckamVaultCtx *vault_ctx,
                                const OckamTransport *transport, OckamTransportCtx transport_ctx) {
  xx->vault = vault;
  xx->vault_ctx = vault_ctx;
  xx->transport = transport;
  xx->transport_ctx = transport_ctx;
  return kOckamErrorNone;
}

OckamError XXEncrypt(KeyEstablishmentXX *xx, uint8_t *payload, uint32_t payload_size, uint8_t *msg, uint16_t msg_length,
                     uint16_t *msg_size) {
  OckamError status = kOckamErrorNone;
  uint8_t cipher_text[MAX_TRANSMIT_SIZE];
  uint8_t vector[VECTOR_SIZE];
  uint32_t offset = 0;

  if (msg_length < (payload_size + TAG_SIZE)) {
    status = kBufferTooSmall;
    goto exit_block;
  }

  memset(cipher_text, 0, sizeof(cipher_text));
  make_vector(xx->ne, vector);
  status =
      xx->vault->AesGcmEncrypt(xx->vault_ctx, xx->ke, KEY_SIZE, vector, sizeof(vector), NULL, 0,
                               &cipher_text[payload_size], TAG_SIZE, payload, payload_size, cipher_text, payload_size);
  if (kOckamErrorNone != status) {
    log_error(status, "failed ockam_vault_aes_gcm_encrypt in encrypt");
    goto exit_block;
  }
  memcpy(msg, cipher_text, TAG_SIZE + payload_size);
  offset += TAG_SIZE + payload_size;
  xx->ne += 1;
  *msg_size = offset;

exit_block:
  return status;
}

OckamError XXDecrypt(KeyEstablishmentXX *xx, uint8_t *payload, uint32_t payload_size, uint8_t *msg, uint16_t msg_length,
                     uint32_t *payload_bytes) {
  OckamError status = kOckamErrorNone;
  uint8_t uncipher[MAX_TRANSMIT_SIZE];
  uint8_t tag[TAG_SIZE];
  uint8_t vector[VECTOR_SIZE];
  uint32_t offset = 0;
  uint32_t uncipher_size = 0;

  if (payload_size < (msg_length - TAG_SIZE)) {
    status = kBufferTooSmall;
    goto exit_block;
  }

  *payload_bytes = msg_length - TAG_SIZE;

  memset(tag, 0, sizeof(tag));
  memcpy(tag, msg + offset + *payload_bytes, TAG_SIZE);
  make_vector(xx->nd, vector);
  memset(uncipher, 0, sizeof(uncipher));
  uncipher_size = msg_length - TAG_SIZE;
  status = xx->vault->AesGcmDecrypt(xx->vault_ctx, xx->kd, KEY_SIZE, vector, sizeof(vector), NULL, 0, tag, sizeof(tag),
                                    msg + offset, uncipher_size, uncipher, uncipher_size);
  if (kOckamErrorNone != status) {
    log_error(status, "failed ockam_vault_aes_gcm_decrypt in initiator_m2_recv");
    goto exit_block;
  }
  memcpy(payload, uncipher, payload_size);
  xx->nd += 1;

exit_block:
  return status;
}

/*
 ********************************************************************************************************
 *                                            LOCAL FUNCTIONS *
 ********************************************************************************************************
 */
OckamError KeyEstablishPrologueXX(KeyEstablishmentXX *xx) {
  OckamError status = kOckamErrorNone;

  // 1. Generate a static 25519 keypair for this handshake and set it to s
  status = xx->vault->KeyGenerate(xx->vault_ctx, kOckamVaultKeyStatic);
  if (kOckamErrorNone != status) {
    log_error(status, "failed to generate static keypair in initiator_step_1");
    goto exit_block;
  }

  status = xx->vault->KeyGetPublic(xx->vault_ctx, kOckamVaultKeyStatic, xx->s, KEY_SIZE);
  if (kOckamErrorNone != status) {
    log_error(status, "failed to generate get static public key in initiator_step_1");
    goto exit_block;
  }

  // 2. Generate an ephemeral 25519 keypair for this handshake and set it to e
  status = xx->vault->KeyGenerate(xx->vault_ctx, kOckamVaultKeyEphemeral);
  if (kOckamErrorNone != status) {
    log_error(status, "failed to generate static keypair in initiator_step_1");
    goto exit_block;
  }

  status = xx->vault->KeyGetPublic(xx->vault_ctx, kOckamVaultKeyEphemeral, xx->e, KEY_SIZE);
  if (kOckamErrorNone != status) {
    log_error(status, "failed to generate get static public key in initiator_step_1");
    goto exit_block;
  }

  // 3. Set k to empty, Set n to 0
  xx->nonce = 0;
  memset(xx->k, 0, KEY_SIZE);

  // 4. Set h and ck to 'Noise_XX_25519_AESGCM_SHA256'
  memset(xx->h, 0, SHA256_SIZE);
  memcpy(xx->h, PROTOCOL_NAME, PROTOCOL_NAME_SIZE);
  memset(xx->ck, 0, KEY_SIZE);
  memcpy(xx->ck, PROTOCOL_NAME, PROTOCOL_NAME_SIZE);

  // 5. h = SHA256(h || prologue),
  // prologue is empty
  mix_hash(xx, NULL, 0);

exit_block:
  return status;
}

/*------------------------------------------------------------------------------------------------------*
 *          UTILITY FUNCTIONS
 *------------------------------------------------------------------------------------------------------*/
void print_uint8_str(uint8_t *p, uint16_t size, char *msg) {
  printf("\n%s %d bytes: \n", msg, size);
  for (int i = 0; i < size; ++i) printf("%0.2x", *p++);
  printf("\n");
}

OckamError HkdfDh(KeyEstablishmentXX *xx, uint8_t *dh1, uint16_t hkdf1_size, OckamVaultKey dh_key_type, uint8_t *dh2,
                  uint16_t dh2_size, uint16_t out_size, uint8_t *out_1, uint8_t *out_2) {
  OckamError status = kOckamErrorNone;
  uint8_t secret[KEY_SIZE];
  uint8_t bytes[2 * out_size];

  // Compute pre-master secret
  // status = ockam_vault_ecdh( dh_key_type, dh2, dh2_size, secret, KEY_SIZE );
  status = xx->vault->Ecdh(xx->vault_ctx, dh_key_type, dh2, dh2_size, secret, KEY_SIZE);
  if (kOckamErrorNone != status) {
    log_error(status, "failed ockam_vault_ecdh in responder_m2_send");
    goto exit_block;
  }

  // ck, k = HKDF( ck, pms )
  status = xx->vault->Hkdf(xx->vault_ctx, dh1, hkdf1_size, secret, KEY_SIZE, NULL, 0, bytes, sizeof(bytes));
  if (kOckamErrorNone != status) {
    log_error(status, "failed ockam_vault_hkdf in responder_m2_send");
    goto exit_block;
  }
  memcpy(out_1, bytes, out_size);
  memcpy(out_2, &bytes[out_size], out_size);

exit_block:
  return status;
}

void string_to_hex(char *hexstring, uint8_t *val, uint32_t *p_bytes) {
  const char *pos = hexstring;
  uint32_t bytes = 0;

  for (size_t count = 0; count < (strlen(hexstring) / 2); count++) {
    sscanf(pos, "%2hhx", &val[count]);
    pos += 2;
    bytes += 1;
  }
  if (NULL != p_bytes) *p_bytes = bytes;
}

void mix_hash(KeyEstablishmentXX *xx, uint8_t *p_bytes, uint16_t b_length) {
  uint8_t *p_h = &xx->h[0];
  uint8_t string[MAX_TRANSMIT_SIZE];
  uint8_t hash[SHA256_SIZE];

  memset(&hash[0], 0, sizeof(hash));
  memset(&string[0], 0, sizeof(string));
  memcpy(&string[0], &p_h[0], SHA256_SIZE);
  memcpy(&string[SHA256_SIZE], p_bytes, b_length);
  xx->vault->Sha256(xx->vault_ctx, (uint8_t *)&string[0], SHA256_SIZE + b_length, (uint8_t *)&hash[0], SHA256_SIZE);
  memcpy(p_h, hash, SHA256_SIZE);

exit_block:
  return;
}

OckamError make_vector(uint64_t nonce, uint8_t *vector) {
  uint8_t *pv;
  uint8_t *pn = (uint8_t *)&nonce;

  memset(vector, 0, VECTOR_SIZE);
  pv = vector + 4;
  pn += 7;
  for (int i = 7; i >= 0; --i) {
    *pv++ = *pn--;
  }
  return kOckamErrorNone;
}

OckamError GetIpInfo(int argc, char *argv[], OckamInternetAddress *p_address) {
  OckamError status = kErrorNone;

  memset(p_address, 0, sizeof(*p_address));

  if (3 != argc) {
    strcpy(p_address->IPAddress, DEFAULT_IP_ADDRESS);
    p_address->port = DEFAULT_IP_PORT;
  } else {
    strcpy(p_address->IPAddress, argv[1]);
    p_address->port = strtoul(argv[2], NULL, 0);
  }

exit_block:
  return status;
}
