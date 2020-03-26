/**
 ********************************************************************************************************
 * @file    xx_responder.c
 * @brief   Interface functions for xx handshake responder
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                             INCLUDE FILES *
 ********************************************************************************************************
 */

#include <string.h>

#include "ockam/error.h"
#include "ockam/key_agreement.h"
#include "ockam/syslog.h"
#include "ockam/transport.h"
#include "ockam/vault.h"
#include "xx_local.h"

/*
 ********************************************************************************************************
 *                                           GLOBAL FUNCTIONS *
 ********************************************************************************************************
 */

OckamError OckamKeyEstablishResponderXX(OckamVault *vault, OckamVaultCtx *vault_ctx, OckamTransport *transport,
                                        OckamTransportCtx transportCtx, KeyEstablishmentXX *xx) {
  OckamError status = kOckamErrorNone;
  uint8_t sendBuffer[MAX_TRANSMIT_SIZE];
  uint8_t readBuffer[MAX_TRANSMIT_SIZE];
  uint16_t bytesReceived = 0;
  uint16_t transmit_size = 0;
  uint8_t compare[1024];
  uint32_t compare_bytes;

  /* Initialize the KeyEstablishmentXX struct */
  memset(xx, 0, sizeof(*xx));
  OckamKeyInitializeXX(xx, vault, vault_ctx, transport, transportCtx);

  /* Initialize handshake struct and generate initial static & ephemeral keys */
  status = KeyEstablishPrologueXX(xx);
  if (kOckamErrorNone != status) {
    log_error(status, "Failed handshake prologue");
    goto exit_block;
  }

  /* Msg 1 receive */
  status = transport->Read(transportCtx, &readBuffer[0], MAX_TRANSMIT_SIZE, &bytesReceived);
  if (status != kErrorNone) {
    log_error(status, "ockam_ReceiveBlocking for msg 1 failed");
    goto exit_block;
  }

  /* Msg 1 process */
  status = XXResponderM1Process(xx, readBuffer, bytesReceived);
  if (status != kErrorNone) {
    log_error(status, "responder_m1_receive failed");
    goto exit_block;
  }

  /* Msg 2 make */
  status = XXResponderM2Make(xx, sendBuffer, sizeof(sendBuffer), &transmit_size);
  if (status != kErrorNone) {
    log_error(status, "responder_m2_send failed");
    goto exit_block;
  }

  /* Msg 2 send */
  status = xx->transport->Write(transportCtx, sendBuffer, transmit_size);
  if (status != kErrorNone) {
    log_error(status, "responder_m2_send failed");
    goto exit_block;
  }

  /* Msg 3 receive */
  status = xx->transport->Read(transportCtx, readBuffer, MAX_TRANSMIT_SIZE, &bytesReceived);
  if (status != kErrorNone) {
    log_error(status, "ockam_ReceiveBlocking failed for msg 3");
    goto exit_block;
  }

  /* Msg 3 process */
  status = XXResponderM3Process(xx, readBuffer, bytesReceived);
  if (status != kErrorNone) {
    log_error(status, "responder_m3_process failed for msg 3");
    goto exit_block;
  }

  /* Epilogue */
  status = XXResponderEpilogue(xx);
  if (kErrorNone != status) {
    log_error(status, "Failed responder_epilogue");
    goto exit_block;
  }

exit_block:
  return status;
}

/*
 ********************************************************************************************************
 *                                            LOCAL FUNCTIONS *
 ********************************************************************************************************
 */

OckamError XXResponderM1Process(KeyEstablishmentXX *xx, uint8_t *p_m1, uint16_t m1_size) {
  OckamError status = kErrorNone;
  uint16_t offset = 0;
  uint8_t key[KEY_SIZE];
  uint32_t key_bytes;

  // Read 32 bytes from the incoming message buffer
  // parse it as a public key, set it to re
  // h = SHA256(h || re)
  memcpy(xx->re, p_m1, KEY_SIZE);
  offset += KEY_SIZE;

  mix_hash(xx, xx->re, KEY_SIZE);

  // h = SHA256( h || payload )
  mix_hash(xx, NULL, 0);

  if (offset != m1_size) {
    status = kXXKeyAgreementFailed;
    log_error(status, "handshake failed in  responder_m1_process (size mismatch)");
  }

exit_block:
  return status;
}

OckamError XXResponderM2Make(KeyEstablishmentXX *xx, uint8_t *p_msg, uint16_t msg_size, uint16_t *p_bytesWritten) {
  OckamError status = kErrorNone;
  uint8_t cipher_text[MAX_TRANSMIT_SIZE];
  uint16_t offset = 0;
  uint8_t vector[VECTOR_SIZE];

  // 1. h = SHA256(h || e.PublicKey),
  // Write e.PublicKey to outgoing message
  // buffer, BigEndian
  mix_hash(xx, xx->e, KEY_SIZE);
  memcpy(p_msg, xx->e, sizeof(xx->e));
  offset += sizeof(xx->e);

  // 2. ck, k = HKDF(ck, DH(e, re), 2)
  // n = 0
  status = HkdfDh(xx, xx->ck, sizeof(xx->ck), kOckamVaultKeyEphemeral, xx->re, sizeof(xx->re), KEY_SIZE, xx->ck, xx->k);
  if (kErrorNone != status) {
    log_error(status, "failed HkdfDh of prologue in responder_m2_make");
    goto exit_block;
  }
  xx->nonce = 0;

  // 3. c = ENCRYPT(k, n++, h, s.PublicKey)
  // h =  SHA256(h || c),
  // Write c to outgoing message buffer
  memset(cipher_text, 0, sizeof(cipher_text));
  make_vector(xx->nonce, vector);
  status = xx->vault->AesGcmEncrypt(xx->vault_ctx, xx->k, KEY_SIZE, vector, sizeof(vector), xx->h, sizeof(xx->h),
                                    &cipher_text[KEY_SIZE], TAG_SIZE, xx->s, KEY_SIZE, cipher_text, KEY_SIZE);
  if (kErrorNone != status) {
    log_error(status, "failed ockam_vault_aes_gcm_encrypt in responder_m2_make");
    goto exit_block;
  }
  xx->nonce += 1;

  mix_hash(xx, cipher_text, KEY_SIZE + TAG_SIZE);

  // Copy cypher text into send buffer
  memcpy(p_msg + offset, cipher_text, KEY_SIZE + TAG_SIZE);
  offset += KEY_SIZE + TAG_SIZE;

  // 4. ck, k = HKDF(ck, DH(s, re), 2)
  // n = 0
  status = HkdfDh(xx, xx->ck, sizeof(xx->ck), kOckamVaultKeyStatic, xx->re, sizeof(xx->re), KEY_SIZE, xx->ck, xx->k);
  if (kErrorNone != status) {
    log_error(status, "failed HkdfDh in responder_m2_make");
    goto exit_block;
  }
  xx->nonce = 0;

  // 5. c = ENCRYPT(k, n++, h, payload)
  // h = SHA256(h || c),
  // payload is empty
  memset(cipher_text, 0, sizeof(cipher_text));
  make_vector(xx->nonce, vector);
  status = xx->vault->AesGcmEncrypt(xx->vault_ctx, xx->k, KEY_SIZE, vector, sizeof(vector), xx->h, sizeof(xx->h),
                                    &cipher_text[0], TAG_SIZE, NULL, 0, NULL, 0);
  if (kErrorNone != status) {
    log_error(status, "failed ockam_vault_aes_gcm_encrypt in responder_m2_make");
    goto exit_block;
  }
  xx->nonce += 1;
  memcpy(p_msg + offset, cipher_text, TAG_SIZE);
  offset += TAG_SIZE;
  mix_hash(xx, cipher_text, TAG_SIZE);

  // Done
  *p_bytesWritten = offset;

exit_block:
  return status;
}

OckamError XXResponderM3Process(KeyEstablishmentXX *xx, uint8_t *p_m3, uint16_t m3_size) {
  OckamError status = kErrorNone;
  uint8_t uncipher[MAX_TRANSMIT_SIZE];
  uint8_t tag[TAG_SIZE];
  uint8_t vector[VECTOR_SIZE];
  uint32_t offset = 0;

  // 1. Read 48 bytes the incoming message buffer as c
  // p = DECRYPT(k, n++, h, c)
  // h = SHA256(h || c),
  // parse p as a public key,
  // set it to rs
  memset(tag, 0, sizeof(tag));
  memcpy(tag, p_m3 + offset + KEY_SIZE, TAG_SIZE);
  make_vector(xx->nonce, vector);
  status = xx->vault->AesGcmDecrypt(xx->vault_ctx, xx->k, KEY_SIZE, vector, sizeof(vector), xx->h, sizeof(xx->h), tag,
                                    sizeof(tag), p_m3, KEY_SIZE, uncipher, KEY_SIZE);

  if (kErrorNone != status) {
    log_error(status, "failed ockam_vault_aes_gcm_decrypt in responder_m3_process");
    goto exit_block;
  }
  memcpy(xx->rs, uncipher, KEY_SIZE);
  mix_hash(xx, p_m3 + offset, KEY_SIZE + TAG_SIZE);
  offset += KEY_SIZE + TAG_SIZE;

  // 2. ck, k = HKDF(ck, DH(e, rs), 2)
  // n = 0
  status = HkdfDh(xx, xx->ck, sizeof(xx->ck), kOckamVaultKeyEphemeral, xx->rs, sizeof(xx->rs), KEY_SIZE, xx->ck, xx->k);
  if (kErrorNone != status) {
    log_error(status, "failed HkdfDh in responder_m3_process");
    goto exit_block;
  }
  xx->nonce = 0;

  // 3. Read remaining bytes of incoming message buffer as c
  // p = DECRYPT(k, n++, h, c)
  // h = SHA256(h || c),
  // parse p as a payload,
  // payload should be empty
  memset(tag, 0, sizeof(tag));
  memcpy(tag, p_m3 + offset, TAG_SIZE);
  make_vector(xx->nonce, vector);
  memset(uncipher, 0, sizeof(uncipher));
  status = xx->vault->AesGcmDecrypt(xx->vault_ctx, xx->k, KEY_SIZE, vector, sizeof(vector), xx->h, sizeof(xx->h), tag,
                                    sizeof(tag), NULL, 0, NULL, 0);
  if (kErrorNone != status) {
    log_error(status, "failed ockam_vault_aes_gcm_decrypt in initiator_m2_recv");
    goto exit_block;
  }
  xx->nonce += 1;
  mix_hash(xx, p_m3 + offset, TAG_SIZE);
  offset += TAG_SIZE;

exit_block:
  return status;
}

OckamError XXResponderEpilogue(KeyEstablishmentXX *xx) {
  OckamError status = kErrorNone;
  uint8_t keys[2 * KEY_SIZE];

  memset(keys, 0, sizeof(keys));
  status = xx->vault->Hkdf(xx->vault_ctx, xx->ck, KEY_SIZE, NULL, 0, NULL, 0, keys, sizeof(keys));
  if (kErrorNone != status) {
    log_error(status, "ockam_vault_hkdf failed in responder_epilogue_make");
    goto exit_block;
  }
  memcpy(xx->ke, keys, KEY_SIZE);
  memcpy(xx->kd, &keys[KEY_SIZE], KEY_SIZE);
  xx->ne = 0;
  xx->nd = 0;

exit_block:
  return status;
}
