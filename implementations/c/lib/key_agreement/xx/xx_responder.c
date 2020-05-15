#include <string.h>

#include "ockam/error.h"
#include "ockam/key_agreement.h"
#include "ockam/syslog.h"
#include "ockam/transport.h"
#include "ockam/vault.h"
#include "xx_local.h"

ockam_error_t ockam_key_establish_responder_xx(key_establishment_xx* xx)
{
  ockam_error_t error = OCKAM_ERROR_NONE;
  uint8_t       write_buffer[MAX_TRANSMIT_SIZE];
  uint8_t       read_buffer[MAX_TRANSMIT_SIZE];
  size_t        bytesReceived = 0;
  size_t        transmit_size = 0;

  /* Initialize handshake struct and generate initial static & ephemeral keys */
  error = key_agreement_prologue_xx(xx);
  if (error) goto exit;

  /* Initialize handshake struct and generate initial static & ephemeral keys */
  error = key_agreement_prologue_xx(xx);
  if (error) goto exit;

  /* Msg 1 receive */
  error = ockam_read(xx->p_reader, read_buffer, MAX_TRANSMIT_SIZE, &bytesReceived);
  if (error) goto exit;

  /* Msg 1 process */
  error = xx_responder_m1_process(xx, read_buffer, bytesReceived);
  if (error) goto exit;

  /* Msg 2 make */
  error = xx_responder_m2_make(xx, write_buffer, sizeof(write_buffer), &transmit_size);
  if (error) goto exit;

  /* Msg 2 send */
  error = ockam_write(xx->p_writer, write_buffer, transmit_size);
  if (error) goto exit;

  /* Msg 3 receive */
  error = ockam_read(xx->p_reader, read_buffer, MAX_TRANSMIT_SIZE, &bytesReceived);
  if (error) goto exit;

  /* Msg 3 process */
  error = xx_responder_m3_process(xx, read_buffer, bytesReceived);
  if (error) goto exit;

  /* Epilogue */
  error = xx_responder_epilogue(xx);
  if (error) goto exit;

exit:
  if (error) log_error(error, __func__);
  return error;
}

/*
 ********************************************************************************************************
 *                                            LOCAL FUNCTIONS *
 ********************************************************************************************************
 */

ockam_error_t xx_responder_m1_process(key_establishment_xx* p_h, uint8_t* p_m1, size_t m1_size)
{
  ockam_error_t error  = TRANSPORT_ERROR_NONE;
  uint16_t      offset = 0;
  uint8_t       key[KEY_SIZE];
  uint32_t      key_bytes;

  // Read 32 bytes from the incoming message buffer
  // parse it as a public key, set it to re
  // h = SHA256(h || re)
  memcpy(p_h->re, p_m1, KEY_SIZE);
  offset += KEY_SIZE;

  mix_hash(p_h, p_h->re, KEY_SIZE);

  // h = SHA256( h || payload )
  mix_hash(p_h, NULL, 0);

  if (offset != m1_size) {
    error = kXXKeyAgreementFailed;
    log_error(error, "handshake failed in  responder_m1_process (size mismatch)");
  }

exit:
  return error;
}

ockam_error_t xx_responder_m2_make(key_establishment_xx* xx, uint8_t* p_msg, size_t msg_size, size_t* p_bytesWritten)
{
  ockam_error_t error = TRANSPORT_ERROR_NONE;
  uint8_t       cipher_and_tag[MAX_TRANSMIT_SIZE];
  size_t        cipher_and_tag_length = 0;
  uint16_t      offset                = 0;
  uint8_t       vector[VECTOR_SIZE];

  // 1. h = SHA256(h || e.PublicKey),
  // Write e.PublicKey to outgoing message
  // buffer, BigEndian
  mix_hash(xx, xx->e, KEY_SIZE);
  memcpy(p_msg, xx->e, sizeof(xx->e));
  offset += sizeof(xx->e);

  // 2. ck, k = HKDF(ck, DH(e, re), 2)
  // n = 0
  error = hkdf_dh(xx, &xx->ck_secret, &xx->e_secret, xx->re, sizeof(xx->re), &xx->ck_secret, &xx->k_secret);
  if (error) goto exit;
  error = ockam_vault_secret_type_set(
    xx->vault, &xx->k_secret, OCKAM_VAULT_SECRET_TYPE_AES256_KEY); //!!Todo: remove these from everywhere
  if (error) goto exit;
  error = ockam_vault_secret_type_set(
    xx->vault, &xx->ck_secret, OCKAM_VAULT_SECRET_TYPE_AES256_KEY); //!!only do this before using for cryptography
  if (error) goto exit;

  xx->nonce = 0;

  // 3. c = ENCRYPT(k, n++, h, s.PublicKey)
  // h =  SHA256(h || c),
  // Write c to outgoing message buffer
  memset(cipher_and_tag, 0, sizeof(cipher_and_tag));
  make_vector(xx->nonce, vector);
  error = ockam_vault_aead_aes_gcm_encrypt(xx->vault,
                                           &xx->k_secret,
                                           xx->nonce,
                                           xx->h,
                                           SHA256_SIZE,
                                           xx->s,
                                           KEY_SIZE,
                                           cipher_and_tag,
                                           KEY_SIZE + TAG_SIZE,
                                           &cipher_and_tag_length);
  if (error) goto exit;

  xx->nonce += 1;
  mix_hash(xx, cipher_and_tag, cipher_and_tag_length);

  // Copy cypher text into send buffer
  memcpy(p_msg + offset, cipher_and_tag, cipher_and_tag_length);
  offset += cipher_and_tag_length;

  // 4. ck, k = HKDF(ck, DH(s, re), 2)
  // n = 0
  error = hkdf_dh(xx, &xx->ck_secret, &xx->s_secret, xx->re, sizeof(xx->re), &xx->ck_secret, &xx->k_secret);
  if (error) goto exit;
  error = ockam_vault_secret_type_set(
    xx->vault, &xx->k_secret, OCKAM_VAULT_SECRET_TYPE_AES256_KEY); //!!Todo: remove these from everywhere
  if (error) goto exit;
  error = ockam_vault_secret_type_set(
    xx->vault, &xx->ck_secret, OCKAM_VAULT_SECRET_TYPE_AES256_KEY); //!!only do this before using for cryptography
  if (error) goto exit;

  xx->nonce = 0;

  // 5. c = ENCRYPT(k, n++, h, payload)
  // h = SHA256(h || c),
  // payload is empty
  memset(cipher_and_tag, 0, sizeof(cipher_and_tag));
  make_vector(xx->nonce, vector);
  error = ockam_vault_aead_aes_gcm_encrypt(xx->vault,
                                           &xx->k_secret,
                                           xx->nonce,
                                           xx->h,
                                           sizeof(xx->h),
                                           NULL,
                                           0,
                                           cipher_and_tag,
                                           sizeof(cipher_and_tag),
                                           &cipher_and_tag_length);

  if (error) goto exit;

  xx->nonce += 1;
  memcpy(p_msg + offset, cipher_and_tag, cipher_and_tag_length);
  offset += cipher_and_tag_length;
  mix_hash(xx, cipher_and_tag, cipher_and_tag_length);

  // Done
  *p_bytesWritten = offset;

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t xx_responder_m3_process(key_establishment_xx* xx, uint8_t* p_m3, size_t m3_size)
{
  ockam_error_t error = TRANSPORT_ERROR_NONE;
  uint8_t       clear_text[MAX_TRANSMIT_SIZE];
  size_t        clear_text_length = 0;
  uint8_t       tag[TAG_SIZE];
  uint32_t      offset = 0;

  // 1. Read 48 bytes the incoming message buffer as c
  // p = DECRYPT(k, n++, h, c)
  // h = SHA256(h || c),
  // parse p as a public key,
  // set it to rs
  memset(tag, 0, sizeof(tag));
  memcpy(tag, p_m3 + offset + KEY_SIZE, TAG_SIZE);
  error = ockam_vault_aead_aes_gcm_decrypt(xx->vault,
                                           &xx->k_secret,
                                           xx->nonce,
                                           xx->h,
                                           sizeof(xx->h),
                                           p_m3,
                                           KEY_SIZE + TAG_SIZE,
                                           clear_text,
                                           sizeof(clear_text),
                                           &clear_text_length);

  if (error) goto exit;

  memcpy(xx->rs, clear_text, KEY_SIZE);
  mix_hash(xx, p_m3 + offset, KEY_SIZE + TAG_SIZE);
  offset += KEY_SIZE + TAG_SIZE;

  // 2. ck, k = HKDF(ck, DH(e, rs), 2)
  // n = 0
  error = hkdf_dh(xx, &xx->ck_secret, &xx->e_secret, xx->rs, sizeof(xx->rs), &xx->ck_secret, &xx->k_secret);
  if (error) goto exit;
  error = ockam_vault_secret_type_set(
    xx->vault, &xx->k_secret, OCKAM_VAULT_SECRET_TYPE_AES256_KEY); //!!Todo: remove these from everywhere
  if (error) goto exit;
  error = ockam_vault_secret_type_set(
    xx->vault, &xx->ck_secret, OCKAM_VAULT_SECRET_TYPE_AES256_KEY); //!!only do this before using for cryptography
  if (error) goto exit;

  xx->nonce = 0;

  // 3. Read remaining bytes of incoming message buffer as c
  // p = DECRYPT(k, n++, h, c)
  // h = SHA256(h || c),
  // parse p as a payload,
  // payload should be empty
  memset(clear_text, 0, sizeof(clear_text));
  error = ockam_vault_aead_aes_gcm_decrypt(xx->vault,
                                           &xx->k_secret,
                                           xx->nonce,
                                           xx->h,
                                           sizeof(xx->h),
                                           p_m3 + offset,
                                           TAG_SIZE,
                                           clear_text,
                                           sizeof(clear_text),
                                           &clear_text_length);
  if (error) goto exit;

  xx->nonce += 1;
  mix_hash(xx, p_m3 + offset, clear_text_length);
  offset += clear_text_length;

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t xx_responder_epilogue(key_establishment_xx* xx)
{
  ockam_error_t        error = TRANSPORT_ERROR_NONE;
  ockam_vault_secret_t secrets[2];

  memset(secrets, 0, sizeof(secrets));
  error = ockam_vault_hkdf_sha256(xx->vault, &xx->ck_secret, NULL, 2, &secrets[0]);
  if (error) goto exit;

  memcpy(&xx->ke_secret, &secrets[0], sizeof(secrets[0]));
  memcpy(&xx->kd_secret, &secrets[1], sizeof(secrets[1]));
  error = ockam_vault_secret_type_set(
    xx->vault, &xx->ke_secret, OCKAM_VAULT_SECRET_TYPE_AES256_KEY); //!!Todo: remove these from everywhere
  if (error) goto exit;
  error = ockam_vault_secret_type_set(
    xx->vault, &xx->kd_secret, OCKAM_VAULT_SECRET_TYPE_AES256_KEY); //!!only do this before using for cryptography
  if (error) goto exit;
  xx->ne = 0;
  xx->nd = 0;

exit:
  if (error) log_error(error, __func__);
  return error;
}
