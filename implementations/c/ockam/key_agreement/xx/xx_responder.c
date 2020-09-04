#include <string.h>

#include "ockam/error.h"
#include "ockam/codec.h"
#include "ockam/key_agreement/xx.h"
#include "ockam/key_agreement/impl.h"
#include "ockam/key_agreement.h"
#include "ockam/log.h"
#include "ockam/transport.h"
#include "ockam/vault.h"
#include "xx_local.h"

#include <stdio.h>

extern ockam_memory_t* gp_ockam_key_memory;

ockam_error_t xx_responder_m1_process(void* ctx, uint8_t* p_m1)
{
  ockam_error_t          error  = ockam_key_agreement_xx_error_none;
  uint16_t               offset = 0;
  ockam_xx_key_t*        key    = (ockam_xx_key_t*) ctx;
  xx_key_exchange_ctx_t* xx     = key->exchange;

  // Read 32 bytes from the incoming message buffer
  // parse it as a public key, set it to re
  // h = SHA256(h || re)
  ockam_memory_copy(gp_ockam_key_memory, xx->re, p_m1, P256_PUBLIC_KEY_SIZE);
  offset += P256_PUBLIC_KEY_SIZE;

  mix_hash(xx, xx->re, P256_PUBLIC_KEY_SIZE);

  // h = SHA256( h || payload )
  mix_hash(xx, NULL, 0);

  // FIXME: error is not used
  return error;
}

ockam_error_t xx_responder_m2_make(void* ctx, uint8_t* p_msg, size_t msg_size, size_t* msg_length)
{
  ockam_error_t          error = ockam_key_agreement_xx_error_none;
  uint8_t                cipher_and_tag[MAX_XX_TRANSMIT_SIZE];
  size_t                 cipher_and_tag_length = 0;
  uint16_t               offset                = 0;
  uint8_t                vector[VECTOR_SIZE];
  ockam_xx_key_t*        key = (ockam_xx_key_t*) ctx;
  xx_key_exchange_ctx_t* xx  = key->exchange;

  // 1. h = SHA256(h || e.PublicKey),
  // Write e.PublicKey to outgoing message
  // buffer, BigEndian
  mix_hash(xx, xx->e, P256_PUBLIC_KEY_SIZE);
  ockam_memory_copy(gp_ockam_key_memory, p_msg + offset, xx->e, sizeof(xx->e));
  offset += sizeof(xx->e);

  // 2. ck, k = HKDF(ck, DH(e, re), 2)
  // n = 0
  error = hkdf_dh(xx, &xx->ck_secret, &xx->e_secret, xx->re, sizeof(xx->re), &xx->ck_secret, &xx->k_secret, false);
  if (ockam_error_has_error(&error)) goto exit;

  xx->nonce = 0;

  // 3. c = ENCRYPT(k, n++, h, s.PublicKey)
  // h =  SHA256(h || c),
  // Write c to outgoing message buffer
  ockam_memory_set(gp_ockam_key_memory, cipher_and_tag, 0, sizeof(cipher_and_tag));
  make_vector(xx->nonce, vector);
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
  mix_hash(xx, cipher_and_tag, cipher_and_tag_length);

  // Copy cypher text into send buffer
  ockam_memory_copy(gp_ockam_key_memory, p_msg + offset, cipher_and_tag, cipher_and_tag_length);
  offset += cipher_and_tag_length;

  // 4. ck, k = HKDF(ck, DH(s, re), 2)
  // n = 0
  error = hkdf_dh(xx, &xx->ck_secret, &xx->s_secret, xx->re, sizeof(xx->re), &xx->ck_secret, &xx->k_secret, false);
  if (ockam_error_has_error(&error)) goto exit;

  xx->nonce = 0;

  // 5. c = ENCRYPT(k, n++, h, payload)
  // h = SHA256(h || c),
  // payload is empty
  ockam_memory_set(gp_ockam_key_memory, cipher_and_tag, 0, sizeof(cipher_and_tag));
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

  if (ockam_error_has_error(&error)) goto exit;

  xx->nonce += 1;
  ockam_memory_copy(gp_ockam_key_memory, p_msg + offset, cipher_and_tag, cipher_and_tag_length);
  offset += cipher_and_tag_length;
  mix_hash(xx, cipher_and_tag, cipher_and_tag_length);

  // Done
  *msg_length = offset;

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t xx_responder_m3_process(void* ctx, uint8_t* p_m3)
{
  ockam_error_t          error = ockam_key_agreement_xx_error_none;
  uint8_t                clear_text[MAX_XX_TRANSMIT_SIZE];
  size_t                 clear_text_length = 0;
  uint32_t               offset = 0;
  ockam_xx_key_t*        key    = (ockam_xx_key_t*) ctx;
  xx_key_exchange_ctx_t* xx     = key->exchange;

  // 1. Read 48 bytes the incoming message buffer as c
  // p = DECRYPT(k, n++, h, c)
  // h = SHA256(h || c),
  // parse p as a public key,
  // set it to rs
//  ockam_memory_set(gp_ockam_key_memory, tag, 0, sizeof(tag));
//  ockam_memory_copy(gp_ockam_key_memory, tag, p_m3 + offset + P256_PUBLIC_KEY_SIZE, TAG_SIZE);

  error = ockam_vault_aead_aes_gcm_decrypt(xx->vault,
                                           &xx->k_secret,
                                           xx->nonce,
                                           xx->h,
                                           SHA256_SIZE,
                                           p_m3,
                                           P256_PUBLIC_KEY_SIZE + TAG_SIZE,
                                           clear_text,
                                           sizeof(clear_text),
                                           &clear_text_length);

  if (ockam_error_has_error(&error)) goto exit;

  ockam_memory_copy(gp_ockam_key_memory, xx->rs, clear_text, P256_PUBLIC_KEY_SIZE);
  mix_hash(xx, p_m3 + offset, P256_PUBLIC_KEY_SIZE + TAG_SIZE);
  offset += P256_PUBLIC_KEY_SIZE + TAG_SIZE;

  // 2. ck, k = HKDF(ck, DH(e, rs), 2)
  // n = 0
  error = hkdf_dh(xx, &xx->ck_secret, &xx->e_secret, xx->rs, sizeof(xx->rs), &xx->ck_secret, &xx->k_secret, true);
  if (ockam_error_has_error(&error)) goto exit;

  xx->nonce = 0;

  // 3. Read remaining bytes of incoming message buffer as c
  // p = DECRYPT(k, n++, h, c)
  // h = SHA256(h || c),
  // parse p as a payload,
  // payload should be empty
  ockam_memory_set(gp_ockam_key_memory, clear_text, 0, sizeof(clear_text));
  error = ockam_vault_aead_aes_gcm_decrypt(xx->vault,
                                           &xx->k_secret,
                                           xx->nonce,
                                           xx->h,
                                           SHA256_SIZE,
                                           p_m3 + offset,
                                           TAG_SIZE,
                                           clear_text,
                                           sizeof(clear_text),
                                           &clear_text_length);
  if (ockam_error_has_error(&error)) goto exit;

  xx->nonce += 1;
  mix_hash(xx, p_m3 + offset, clear_text_length + TAG_SIZE);

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t xx_responder_epilogue(ockam_key_t* key)
{
  ockam_error_t          error = ockam_key_agreement_xx_error_none;
  ockam_vault_secret_t   secrets[2];
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

  error = ockam_vault_hkdf_sha256(exchange_ctx->vault, &exchange_ctx->ck_secret, NULL, 2, &secrets[0]);
  if (ockam_error_has_error(&error)) goto exit;

  ockam_memory_copy(gp_ockam_key_memory, &xx_key->encrypt_secret, &secrets[0], sizeof(secrets[0]));
  ockam_memory_copy(gp_ockam_key_memory, &xx_key->decrypt_secret, &secrets[1], sizeof(secrets[1]));
  error = ockam_vault_secret_type_set(exchange_ctx->vault, &xx_key->encrypt_secret, OCKAM_VAULT_SECRET_TYPE_AES128_KEY);
  if (ockam_error_has_error(&error)) goto exit;
  error = ockam_vault_secret_type_set(exchange_ctx->vault, &xx_key->decrypt_secret, OCKAM_VAULT_SECRET_TYPE_AES128_KEY);
  if (ockam_error_has_error(&error)) goto exit;
  xx_key->encrypt_nonce = 0;
  xx_key->decrypt_nonce = 0;

  for (int i = 0; i < sizeof(xx_key->h); ++i) xx_key->h[i] = exchange_ctx->h[i];

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}