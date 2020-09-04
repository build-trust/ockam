#include <string.h>

#include "ockam/error.h"
#include "ockam/key_agreement/xx.h"
#include "ockam/key_agreement/impl.h"
#include "ockam/key_agreement.h"
#include "ockam/log.h"
#include "ockam/vault.h"
#include "ockam/codec.h"
#include "xx_local.h"

extern ockam_memory_t* gp_ockam_key_memory;
uint8_t                clear_text[MAX_XX_TRANSMIT_SIZE];

ockam_error_t xx_initiator_m1_make(void* ctx, uint8_t* p_send_buffer, size_t buffer_size, size_t* p_transmit_size)
{
  ockam_error_t          error  = ockam_key_agreement_xx_error_none;
  uint16_t               offset = 0;
  ockam_xx_key_t*        key    = (ockam_xx_key_t*) ctx;
  xx_key_exchange_ctx_t* xx     = key->exchange;

  // Write e to outgoing buffer
  // h = SHA256(h || e.PublicKey
  ockam_memory_copy(gp_ockam_key_memory, p_send_buffer, xx->e, P256_PUBLIC_KEY_SIZE);
  offset += P256_PUBLIC_KEY_SIZE;

  mix_hash(xx, xx->e, sizeof(xx->e));

  // Write payload to outgoing buffer, payload is empty
  // h = SHA256( h || payload )
  mix_hash(xx, NULL, 0);

  *p_transmit_size = offset;

  return error;
}

ockam_error_t xx_initiator_m2_process(void* ctx, uint8_t* recv_buffer)
{
  ockam_error_t          error             = ockam_key_agreement_xx_error_none;
  uint16_t               offset            = 0;
  size_t                 clear_text_length = 0;
  ockam_xx_key_t*        key               = (ockam_xx_key_t*) ctx;
  xx_key_exchange_ctx_t* xx                = key->exchange;

  // 1. Read 32 bytes from the incoming
  // message buffer, parse it as a public
  // key, set it to re
  // h = SHA256(h || re)
  // FIXME: Check size
  ockam_memory_copy(gp_ockam_key_memory, xx->re, recv_buffer, P256_PUBLIC_KEY_SIZE);
  offset += P256_PUBLIC_KEY_SIZE;
  mix_hash(xx, xx->re, sizeof(xx->re));

  // 2. ck, k = HKDF(ck, DH(e, re), 2)
  // n = 0
  error = hkdf_dh(xx, &xx->ck_secret, &xx->e_secret, xx->re, sizeof(xx->re), &xx->ck_secret, &xx->k_secret, false);
  if (ockam_error_has_error(&error)) goto exit;

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
                                           recv_buffer + offset,
                                           P256_PUBLIC_KEY_SIZE + TAG_SIZE,
                                           clear_text,
                                           sizeof(clear_text),
                                           &clear_text_length);
  if (ockam_error_has_error(&error)) goto exit;

  xx->nonce += 1;
  ockam_memory_copy(gp_ockam_key_memory, xx->rs, clear_text, P256_PUBLIC_KEY_SIZE);
  mix_hash(xx, recv_buffer + offset, P256_PUBLIC_KEY_SIZE + TAG_SIZE);
  offset += P256_PUBLIC_KEY_SIZE + TAG_SIZE;

  // 4. ck, k = HKDF(ck, DH(e, rs), 2)
  // n = 0
  // secret = ECDH( e, re )
  error = hkdf_dh(xx, &xx->ck_secret, &xx->e_secret, xx->rs, sizeof(xx->rs), &xx->ck_secret, &xx->k_secret, false);
  if (ockam_error_has_error(&error)) goto exit;

  xx->nonce = 0;

  // 5. Read remaining bytes of incoming
  // message buffer as c
  // p = DECRYPT(k, n++, h, c)
  // h = SHA256(h || c),
  // parse p as a payload,
  // payload should be empty
  xx->nonce += 1;
  mix_hash(xx, recv_buffer + offset, TAG_SIZE);

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t xx_initiator_m3_make(void* ctx, uint8_t* p_msg, size_t msg_size, size_t* p_msg_length)
{
  ockam_error_t error = ockam_key_agreement_xx_error_none;
  uint8_t       cipher_and_tag[P256_PUBLIC_KEY_SIZE + TAG_SIZE];
  size_t        cipher_and_tag_length = 0;
  u_int16_t     offset                = 0;
  ockam_xx_key_t*        key = (ockam_xx_key_t*) ctx;
  xx_key_exchange_ctx_t* xx  = key->exchange;

  // 1. c = ENCRYPT(k, n++, h, s.PublicKey)
  // h =  SHA256(h || c),
  // Write c to outgoing message
  // buffer, BigEndian
  ockam_memory_set(gp_ockam_key_memory, cipher_and_tag, 0, sizeof(cipher_and_tag));
  error = ockam_vault_aead_aes_gcm_encrypt(xx->vault,
                                           &xx->k_secret,
                                           xx->nonce,
                                           xx->h,
                                           SHA256_SIZE,
                                           xx->s,
                                           P256_PUBLIC_KEY_SIZE,
                                           cipher_and_tag,
                                           P256_PUBLIC_KEY_SIZE + TAG_SIZE,
                                           &cipher_and_tag_length);
  if (ockam_error_has_error(&error)) goto exit;

  xx->nonce += 1;

  ockam_memory_copy(gp_ockam_key_memory, p_msg, cipher_and_tag, P256_PUBLIC_KEY_SIZE + TAG_SIZE);
  offset += cipher_and_tag_length;
  mix_hash(xx, p_msg, P256_PUBLIC_KEY_SIZE + TAG_SIZE);

  // 2. ck, k = HKDF(ck, DH(s, re), 2)
  // n = 0
  error = hkdf_dh(xx, &xx->ck_secret, &xx->s_secret, xx->re, sizeof(xx->re), &xx->ck_secret, &xx->k_secret, true);
  if (ockam_error_has_error(&error)) goto exit;

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
                                           P256_PUBLIC_KEY_SIZE + TAG_SIZE,
                                           &cipher_and_tag_length);
  if (ockam_error_has_error(&error)) goto exit;

  xx->nonce += 1;
  mix_hash(xx, cipher_and_tag, cipher_and_tag_length);
  ockam_memory_copy(gp_ockam_key_memory, p_msg + offset, cipher_and_tag, cipher_and_tag_length);
  offset += cipher_and_tag_length;

  *p_msg_length = offset;

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t xx_initiator_epilogue(ockam_key_t* key)
{
  ockam_error_t          error        = ockam_key_agreement_xx_error_none;
  ockam_vault_secret_t   secrets[2]   = {0};
  ockam_xx_key_t*        xx_key       = (ockam_xx_key_t*) key->context;
  xx_key_exchange_ctx_t* exchange_ctx = xx_key->exchange;

  ockam_memory_set(gp_ockam_key_memory, secrets, 0, sizeof(secrets));

  ockam_vault_secret_attributes_t attributes = {
    .length = SYMMETRIC_KEY_SIZE,
    .type = OCKAM_VAULT_SECRET_TYPE_AES128_KEY,
    .purpose = OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT,
    .persistence = OCKAM_VAULT_SECRET_EPHEMERAL,
  };

  secrets[0].attributes = attributes;
  secrets[1].attributes = attributes;

  error = ockam_vault_hkdf_sha256(xx_key->vault, &exchange_ctx->ck_secret, NULL, 2, secrets);
  if (ockam_error_has_error(&error)) goto exit;

  ockam_memory_copy(gp_ockam_key_memory, &xx_key->decrypt_secret, &secrets[0], sizeof(secrets[0]));
  ockam_memory_copy(gp_ockam_key_memory, &xx_key->encrypt_secret, &secrets[1], sizeof(secrets[1]));

  xx_key->encrypt_nonce = 0;
  xx_key->decrypt_nonce = 0;

  for (int i = 0; i < sizeof(xx_key->h); ++i) xx_key->h[i] = exchange_ctx->h[i];

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}
