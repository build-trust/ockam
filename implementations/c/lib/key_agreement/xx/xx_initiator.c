/**
 ********************************************************************************************************
 * @file    xx_responder.c
 * @brief   Interface functions for xx handshake responder
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
OckamError OckamKeyEstablishInitiatorXX(OckamVault *vault, OckamVaultCtx *vault_ctx, OckamTransport *transport,
                                        OckamTransportCtx transport_ctx, KeyEstablishmentXX *xx) {
  OckamError status = kOckamErrorNone;
  uint8_t sendBuffer[MAX_TRANSMIT_SIZE];
  uint8_t recv_buffer[MAX_TRANSMIT_SIZE];
  uint16_t bytesReceived = 0;
  uint16_t transmit_size = 0;
  uint8_t compare[1024];
  uint32_t compare_bytes;

  /* Initialize the KeyEstablishmentXX struct */
  memset(xx, 0, sizeof(*xx));
  OckamKeyInitializeXX(xx, vault, vault_ctx, transport, transport_ctx);

  /* Initialize handshake struct and generate initial static & ephemeral keys */
  status = KeyEstablishPrologueXX(xx);
  if (kOckamErrorNone != status) {
    log_error(status, "Failed handshake prologue");
    goto exit_block;
  }

  // Step 1 generate message
  status = XXInitiatorM1Make(xx, sendBuffer, MAX_TRANSMIT_SIZE, &transmit_size);
  if (kOckamErrorNone != status) {
    log_error(status, "initiator_step_1 failed");
    goto exit_block;
  }

  // Step 1 send message
  status = transport->Write(transport_ctx, sendBuffer, transmit_size);
  if (kOckamErrorNone != status) {
    log_error(status, "ockam_SendBlocking after initiator_step_1 failed");
    goto exit_block;
  }

  // Msg 2 receive
  status = transport->Read(transport_ctx, recv_buffer, sizeof(recv_buffer), &bytesReceived);
  if (kOckamErrorNone != status) {
    log_error(status, "ockam_ReceiveBlocking failed on msg 2");
    goto exit_block;
  }

  // Msg 2 process
  status = XXInitiatorM2Process(xx, recv_buffer, bytesReceived);
  if (kOckamErrorNone != status) {
    log_error(status, "XXInitiatorM2Process failed on msg 2");
    goto exit_block;
  }

  // Msg 3 make
  status = XXInitiatorM3Make(xx, sendBuffer, &transmit_size);
  if (kOckamErrorNone != status) {
    log_error(status, "initiator_m3_make failed");
    goto exit_block;
  }

  // Msg 3 send
  status = transport->Write(transport_ctx, sendBuffer, transmit_size);
  if (kOckamErrorNone != status) {
    log_error(status, "ockam_SendBlocking failed on msg 3");
    goto exit_block;
  }

  status = XXInitiatorEpilogue(xx);
  if (kOckamErrorNone != status) {
    log_error(status, "initiator_epilogue failed");
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

/*------------------------------------------------------------------------------------------------------*
 *          INITIATOR FUNCTIONS
 *------------------------------------------------------------------------------------------------------*/

OckamError XXInitiatorM1Make(KeyEstablishmentXX *xx, uint8_t *p_sendBuffer, uint16_t buffer_length,
                             uint16_t *p_transmit_size) {
  OckamError status = kOckamErrorNone;
  uint16_t offset = 0;

  // Write e to outgoing buffer
  // h = SHA256(h || e.PublicKey
  memcpy(p_sendBuffer, xx->e, KEY_SIZE);
  offset += KEY_SIZE;

  mix_hash(xx, xx->e, sizeof(xx->e));

  // Write payload to outgoing buffer, payload is empty
  // h = SHA256( h || payload )
  mix_hash(xx, NULL, 0);

  *p_transmit_size = offset;

  return status;
}

OckamError XXInitiatorM2Process(KeyEstablishmentXX *xx, uint8_t *p_recv, uint16_t recv_size) {
  OckamError status = kOckamErrorNone;
  uint16_t offset = 0;
  uint8_t uncipher[MAX_TRANSMIT_SIZE];
  uint8_t tag[TAG_SIZE];
  uint8_t vector[VECTOR_SIZE];

  // 1. Read 32 bytes from the incoming
  // message buffer, parse it as a public
  // key, set it to re
  // h = SHA256(h || re)
  memcpy(xx->re, p_recv, KEY_SIZE);
  offset += KEY_SIZE;
  mix_hash(xx, xx->re, KEY_SIZE);

  // 2. ck, k = HKDF(ck, DH(e, re), 2)
  // n = 0
  status = HkdfDh(xx, xx->ck, sizeof(xx->ck), kOckamVaultKeyEphemeral, xx->re, sizeof(xx->re), KEY_SIZE, xx->ck, xx->k);
  if (kOckamErrorNone != status) {
    log_error(status, "failed HkdfDh of prologue in responder_m2_make");
    goto exit_block;
  }
  xx->nonce = 0;

  // 3. Read 48 bytes of the incoming message buffer as c
  // p = DECRYPT(k, n++, h, c)
  // h = SHA256(h || c),
  // parse p as a public key,
  // set it to rs
  memset(tag, 0, sizeof(tag));
  memcpy(tag, p_recv + offset + KEY_SIZE, TAG_SIZE);
  make_vector(xx->nonce, vector);
  status = xx->vault->AesGcmDecrypt(xx->vault_ctx, xx->k, KEY_SIZE, vector, sizeof(vector), xx->h, sizeof(xx->h), tag,
                                    sizeof(tag), p_recv + offset, KEY_SIZE, uncipher, KEY_SIZE);
  if (kOckamErrorNone != status) {
    log_error(status, "failed ockam_vault_aes_gcm_decrypt in initiator_m2_recv");
    goto exit_block;
  }
  xx->nonce += 1;
  memcpy(xx->rs, uncipher, KEY_SIZE);
  mix_hash(xx, p_recv + offset, KEY_SIZE + TAG_SIZE);
  offset += KEY_SIZE + TAG_SIZE;

  // 4. ck, k = HKDF(ck, DH(e, rs), 2)
  // n = 0
  // secret = ECDH( e, re )
  status = HkdfDh(xx, xx->ck, sizeof(xx->ck), kOckamVaultKeyEphemeral, xx->rs, sizeof(xx->rs), KEY_SIZE, xx->ck, xx->k);
  if (kOckamErrorNone != status) {
    log_error(status, "failed HkdfDh of prologue in initiator_m2_process");
    goto exit_block;
  }
  xx->nonce = 0;

  // 5. Read remaining bytes of incoming
  // message buffer as c
  // p = DECRYPT(k, n++, h, c)
  // h = SHA256(h || c),
  // parse p as a payload,
  // payload should be empty
  memset(tag, 0, sizeof(tag));
  memcpy(tag, p_recv + offset, TAG_SIZE);
  make_vector(xx->nonce, vector);
  status = xx->vault->AesGcmDecrypt(xx->vault_ctx, xx->k, KEY_SIZE, vector, sizeof(vector), xx->h, sizeof(xx->h), tag,
                                    sizeof(tag), NULL, 0, NULL, 0);
  if (kOckamErrorNone != status) {
    log_error(status, "failed ockam_vault_aes_gcm_decrypt in initiator_m2_recv");
    goto exit_block;
  }
  xx->nonce += 1;
  mix_hash(xx, p_recv + offset, TAG_SIZE);

exit_block:
  return status;
}

OckamError XXInitiatorM3Make(KeyEstablishmentXX *xx, uint8_t *p_msg, uint16_t *p_msg_size) {
  OckamError status = kOckamErrorNone;
  uint8_t tag[TAG_SIZE];
  uint8_t cipher[KEY_SIZE];
  u_int16_t offset = 0;
  uint8_t vector[VECTOR_SIZE];

  // 1. c = ENCRYPT(k, n++, h, s.PublicKey)
  // h =  SHA256(h || c),
  // Write c to outgoing message
  // buffer, BigEndian
  memset(cipher, 0, sizeof(cipher));
  make_vector(xx->nonce, vector);
  status = xx->vault->AesGcmEncrypt(xx->vault_ctx, xx->k, KEY_SIZE, vector, sizeof(vector), xx->h, SHA256_SIZE, tag,
                                    TAG_SIZE, xx->s, KEY_SIZE, cipher, KEY_SIZE);
  if (kOckamErrorNone != status) {
    log_error(status, "failed ockam_vault_aes_gcm_encrypt in initiator_m3_make");
    goto exit_block;
  }
  xx->nonce += 1;
  memcpy(p_msg, cipher, KEY_SIZE);
  offset += KEY_SIZE;
  memcpy(p_msg + offset, tag, TAG_SIZE);
  offset += TAG_SIZE;
  mix_hash(xx, p_msg, KEY_SIZE + TAG_SIZE);

  // 2. ck, k = HKDF(ck, DH(s, re), 2)
  // n = 0
  status = HkdfDh(xx, xx->ck, sizeof(xx->ck), kOckamVaultKeyStatic, xx->re, sizeof(xx->re), KEY_SIZE, xx->ck, xx->k);
  if (kOckamErrorNone != status) {
    log_error(status, "failed HkdfDh in initiator_m3_make");
    goto exit_block;
  }
  xx->nonce = 0;

  // 3. c = ENCRYPT(k, n++, h, payload)
  // h = SHA256(h || c),
  // payload is empty
  make_vector(xx->nonce, vector);
  status = xx->vault->AesGcmEncrypt(xx->vault_ctx, xx->k, KEY_SIZE, vector, sizeof(vector), xx->h, sizeof(xx->h),
                                    cipher, TAG_SIZE, NULL, 0, NULL, 0);

  if (kOckamErrorNone != status) {
    log_error(status, "failed HkdfDh in initiator_m3_make");
    goto exit_block;
  }
  xx->nonce += 1;
  mix_hash(xx, cipher, TAG_SIZE);
  memcpy(p_msg + offset, cipher, TAG_SIZE);
  offset += TAG_SIZE;
  // Copy cipher text into send buffer, append tag

  *p_msg_size = offset;

exit_block:
  return status;
}

OckamError XXInitiatorEpilogue(KeyEstablishmentXX *xx) {
  OckamError status = kOckamErrorNone;
  uint8_t keys[2 * KEY_SIZE];

  memset(keys, 0, sizeof(keys));
  status = xx->vault->Hkdf(xx->vault_ctx, xx->ck, KEY_SIZE, NULL, 0, NULL, 0, keys, sizeof(keys));
  if (kOckamErrorNone != status) {
    log_error(status, "ockam_vault_hkdf failed in responder_epilogue_make");
  }
  memcpy(xx->kd, keys, KEY_SIZE);
  memcpy(xx->ke, &keys[KEY_SIZE], KEY_SIZE);
  xx->ne = 0;
  xx->nd = 0;

exit_block:
  return status;
}
