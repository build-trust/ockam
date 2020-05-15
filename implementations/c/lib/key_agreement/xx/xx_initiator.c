#include <string.h>

#include "ockam/error.h"
#include "ockam/key_agreement.h"
#include "ockam/syslog.h"
#include "ockam/transport.h"
#include "ockam/vault.h"
#include "xx_local.h"

ockam_error_t ockam_key_establish_initiator_xx(key_establishment_xx* xx)
{
  ockam_error_t error = OCKAM_ERROR_NONE;
  uint8_t       send_buffer[MAX_TRANSMIT_SIZE];
  uint8_t       recv_buffer[MAX_TRANSMIT_SIZE];
  size_t        bytes_received = 0;
  size_t        transmit_size  = 0;
  uint8_t       compare[1024];
  size_t        compare_bytes;

  /* Initialize handshake struct and generate initial static & ephemeral keys */
  error = key_agreement_prologue_xx(xx);
  if (error) goto exit;

  error = xx_initiator_m1_make(xx, send_buffer, MAX_TRANSMIT_SIZE, &transmit_size);
  if (error) goto exit;

  error = ockam_write(xx->p_writer, send_buffer, transmit_size);
  if (error) goto exit;

  error = ockam_read(xx->p_reader, recv_buffer, sizeof(recv_buffer), &bytes_received);
  if (error) goto exit;

  error = xx_initiator_m2_process(xx, recv_buffer, bytes_received);
  if (error) goto exit;

  error = xx_initiator_m3_make(xx, send_buffer, &transmit_size);
  if (error) goto exit;

  error = ockam_write(xx->p_writer, send_buffer, transmit_size);
  if (error) goto exit;

  error = xx_initiator_epilogue(xx);
  if (error) goto exit;

exit:
  if (error) log_error(error, __func__);
  return error;
}

/*------------------------------------------------------------------------------------------------------*
 *          INITIATOR FUNCTIONS
 *------------------------------------------------------------------------------------------------------*/

ockam_error_t
xx_initiator_m1_make(key_establishment_xx* xx, uint8_t* p_send_buffer, size_t buffer_size, size_t* p_transmit_size)
{
  ockam_error_t error  = OCKAM_ERROR_NONE;
  uint16_t      offset = 0;

  // Write e to outgoing buffer
  // h = SHA256(h || e.PublicKey
  memcpy(p_send_buffer, xx->e, KEY_SIZE);
  offset += KEY_SIZE;

  mix_hash(xx, xx->e, sizeof(xx->e));

  // Write payload to outgoing buffer, payload is empty
  // h = SHA256( h || payload )
  mix_hash(xx, NULL, 0);

  *p_transmit_size = offset;

  return error;
}

ockam_error_t xx_initiator_m2_process(key_establishment_xx* xx, uint8_t* p_recv, size_t recv_size)
{
  ockam_error_t        error  = OCKAM_ERROR_NONE;
  uint16_t             offset = 0;
  uint8_t              clear_text[MAX_TRANSMIT_SIZE];
  size_t               clear_text_length = 0;
  uint8_t              tag[TAG_SIZE];
  uint8_t              vector[VECTOR_SIZE];
  ockam_vault_secret_t secrets[2];

  // 1. Read 32 bytes from the incoming
  // message buffer, parse it as a public
  // key, set it to re
  // h = SHA256(h || re)
  memcpy(xx->re, p_recv, KEY_SIZE);
  offset += KEY_SIZE;
  mix_hash(xx, xx->re, KEY_SIZE);

  // 2. ck, k = HKDF(ck, DH(e, re), 2)
  // n = 0
  error = hkdf_dh(xx, &xx->ck_secret, &xx->e_secret, xx->re, sizeof(xx->re), &xx->ck_secret, &xx->k_secret);
  if (error) goto exit;

  error = ockam_vault_secret_type_set(xx->vault, &xx->k_secret, OCKAM_VAULT_SECRET_TYPE_AES256_KEY);
  if (error) goto exit;
  error = ockam_vault_secret_type_set(xx->vault, &xx->ck_secret, OCKAM_VAULT_SECRET_TYPE_AES256_KEY);
  if (error) goto exit;
  xx->nonce = 0;

  // 3. Read 48 bytes of the incoming message buffer as c
  // p = DECRYPT(k, n++, h, c)
  // h = SHA256(h || c),
  // parse p as a public key,
  // set it to rs
  error = ockam_vault_aead_aes_gcm_decrypt(xx->vault,
                                           &xx->k_secret,
                                           xx->nonce,
                                           xx->h,
                                           sizeof(xx->h),
                                           p_recv + offset,
                                           KEY_SIZE + TAG_SIZE,
                                           clear_text,
                                           sizeof(clear_text),
                                           &clear_text_length);
  if (error) goto exit;

  xx->nonce += 1;
  memcpy(xx->rs, clear_text, KEY_SIZE);
  mix_hash(xx, p_recv + offset, KEY_SIZE + TAG_SIZE);
  offset += KEY_SIZE + TAG_SIZE;

  // 4. ck, k = HKDF(ck, DH(e, rs), 2)
  // n = 0
  // secret = ECDH( e, re )
  error = hkdf_dh(xx, &xx->ck_secret, &xx->e_secret, xx->rs, sizeof(xx->rs), &xx->ck_secret, &xx->k_secret);
  if (error) goto exit;

  error = ockam_vault_secret_type_set(xx->vault, &xx->k_secret, OCKAM_VAULT_SECRET_TYPE_AES256_KEY);
  if (error) goto exit;
  error = ockam_vault_secret_type_set(xx->vault, &xx->ck_secret, OCKAM_VAULT_SECRET_TYPE_AES256_KEY);
  if (error) goto exit;

  xx->nonce = 0;

  // 5. Read remaining bytes of incoming
  // message buffer as c
  // p = DECRYPT(k, n++, h, c)
  // h = SHA256(h || c),
  // parse p as a payload,
  // payload should be empty
  xx->nonce += 1;
  mix_hash(xx, p_recv + offset, TAG_SIZE);

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t xx_initiator_m3_make(key_establishment_xx* xx, uint8_t* p_msg, size_t* p_msg_size)
{
  ockam_error_t error = OCKAM_ERROR_NONE;
  uint8_t       cipher_and_tag[KEY_SIZE + TAG_SIZE];
  size_t        cipher_and_tag_length = 0;
  u_int16_t     offset                = 0;
  uint8_t       vector[VECTOR_SIZE];

  // 1. c = ENCRYPT(k, n++, h, s.PublicKey)
  // h =  SHA256(h || c),
  // Write c to outgoing message
  // buffer, BigEndian
  memset(cipher_and_tag, 0, sizeof(cipher_and_tag));
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
  memcpy(p_msg, cipher_and_tag, KEY_SIZE + TAG_SIZE);
  offset += KEY_SIZE + TAG_SIZE;
  mix_hash(xx, p_msg, KEY_SIZE + TAG_SIZE);

  // 2. ck, k = HKDF(ck, DH(s, re), 2)
  // n = 0
  error = hkdf_dh(xx, &xx->ck_secret, &xx->s_secret, xx->re, sizeof(xx->re), &xx->ck_secret, &xx->k_secret);
  if (error) goto exit;

  error = ockam_vault_secret_type_set(xx->vault, &xx->k_secret, OCKAM_VAULT_SECRET_TYPE_AES256_KEY);
  if (error) goto exit;
  error = ockam_vault_secret_type_set(xx->vault, &xx->ck_secret, OCKAM_VAULT_SECRET_TYPE_AES256_KEY);
  if (error) goto exit;

  xx->nonce = 0;

  // 3. c = ENCRYPT(k, n++, h, payload)
  // h = SHA256(h || c),
  // payload is empty
  error = ockam_vault_aead_aes_gcm_encrypt(xx->vault,
                                           &xx->k_secret,
                                           xx->nonce,
                                           xx->h,
                                           SHA256_SIZE,
                                           NULL,
                                           0,
                                           cipher_and_tag,
                                           KEY_SIZE + TAG_SIZE,
                                           &cipher_and_tag_length);
  if (error) goto exit;

  xx->nonce += 1;
  mix_hash(xx, cipher_and_tag, cipher_and_tag_length);
  memcpy(p_msg + offset, cipher_and_tag, cipher_and_tag_length);
  offset += cipher_and_tag_length;

  *p_msg_size = offset;

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t xx_initiator_epilogue(key_establishment_xx* xx)
{
  ockam_error_t        error = OCKAM_ERROR_NONE;
  ockam_vault_secret_t secrets[2];

  memset(secrets, 0, sizeof(secrets));
  error = ockam_vault_hkdf_sha256(xx->vault, &xx->ck_secret, NULL, 2, secrets);
  if (error) goto exit;

  memcpy(&xx->kd_secret, &secrets[0], sizeof(secrets[0]));
  memcpy(&xx->ke_secret, &secrets[1], sizeof(secrets[1]));

  error = ockam_vault_secret_type_set(xx->vault, &xx->kd_secret, OCKAM_VAULT_SECRET_TYPE_AES256_KEY);
  if (error) goto exit;
  error = ockam_vault_secret_type_set(xx->vault, &xx->ke_secret, OCKAM_VAULT_SECRET_TYPE_AES256_KEY);
  if (error) goto exit;

  xx->nonce = 0;
  xx->ne    = 0;
  xx->nd    = 0;

exit:
  if (error) log_error(error, __func__);
  return error;
}
