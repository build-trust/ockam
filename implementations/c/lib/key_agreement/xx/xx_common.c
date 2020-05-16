#include <string.h>
#include <stdio.h>

#include "ockam/error.h"
#include "ockam/error.h"
#include "ockam/key_agreement.h"
#include "ockam/syslog.h"
#include "ockam/vault.h"
#include "../../vault/default/default.h"
#include "xx_local.h"

/*
 ********************************************************************************************************
 *                                           GLOBAL FUNCTIONS *
 ********************************************************************************************************
 */

ockam_error_t xx_encrypt(
  key_establishment_xx* xx, uint8_t* payload, size_t payload_size, uint8_t* msg, size_t msg_length, size_t* msg_size)
{
  ockam_error_t error = OCKAM_ERROR_NONE;
  uint8_t       cipher_text_and_tag[MAX_TRANSMIT_SIZE];
  size_t        ciphertext_and_tag_length;

  if (msg_length < (payload_size + TAG_SIZE)) {
    error = TRANSPORT_ERROR_BUFFER_TOO_SMALL;
    goto exit;
  }

  memset(cipher_text_and_tag, 0, sizeof(cipher_text_and_tag));
  error = ockam_vault_aead_aes_gcm_encrypt(xx->vault,
                                           &xx->ke_secret,
                                           xx->ne,
                                           NULL,
                                           0,
                                           payload,
                                           payload_size,
                                           cipher_text_and_tag,
                                           sizeof(cipher_text_and_tag),
                                           &ciphertext_and_tag_length);
  ;
  memcpy(msg, cipher_text_and_tag, ciphertext_and_tag_length);
  xx->ne += 1;
  *msg_size = ciphertext_and_tag_length;

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t xx_decrypt(key_establishment_xx* xx,
                         uint8_t*              payload,
                         size_t                payload_size,
                         uint8_t*              cipher_text,
                         size_t                cipher_text_length,
                         size_t*               payload_length)
{
  ockam_error_t error = OCKAM_ERROR_NONE;
  uint8_t       clear_text[MAX_TRANSMIT_SIZE];
  size_t        clear_text_length = 0;

  memset(clear_text, 0, sizeof(clear_text));

  error           = ockam_vault_aead_aes_gcm_decrypt(xx->vault,
                                           &xx->kd_secret,
                                           xx->nd,
                                           NULL,
                                           0,
                                           cipher_text,
                                           cipher_text_length,
                                           clear_text,
                                           sizeof(clear_text),
                                           &clear_text_length);
  *payload_length = clear_text_length;

  memcpy(payload, clear_text, clear_text_length);
  xx->nd += 1;

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t xx_key_deinit(key_establishment_xx* xx)
{
  ockam_error_t error        = OCKAM_ERROR_NONE;
  ockam_error_t return_error = OCKAM_ERROR_NONE;
  error                      = ockam_vault_secret_destroy(xx->vault, &xx->e_secret);
  if (error) return_error = error;
  error = ockam_vault_secret_destroy(xx->vault, &xx->s_secret);
  if (error) return_error = error;
  error = ockam_vault_secret_destroy(xx->vault, &xx->ke_secret);
  if (error) return_error = error;
  error = ockam_vault_secret_destroy(xx->vault, &xx->kd_secret);
  if (error) return_error = error;
  error = ockam_vault_secret_destroy(xx->vault, &xx->k_secret);
  if (error) return_error = error;
  error = ockam_vault_secret_destroy(xx->vault, &xx->ck_secret);
  if (error) return_error = error;
exit:
  return return_error;
}

/*
 ********************************************************************************************************
 *                                            LOCAL FUNCTIONS *
 ********************************************************************************************************
 */
ockam_error_t key_agreement_prologue_xx(key_establishment_xx* xx)
{
  ockam_error_t                   error             = OCKAM_ERROR_NONE;
  ockam_vault_secret_attributes_t secret_attributes = { KEY_SIZE,
                                                        OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY,
                                                        OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT,
                                                        OCKAM_VAULT_SECRET_EPHEMERAL };
  size_t                          key_size          = 0;
  uint8_t                         ck[KEY_SIZE];

  // 1. Generate a static 25519 keypair for this handshake and set it to s
  error = ockam_vault_secret_generate(xx->vault, &xx->s_secret, &secret_attributes);
  if (error) goto exit;

  error = ockam_vault_secret_publickey_get(xx->vault, &xx->s_secret, xx->s, sizeof(xx->s), &key_size);
  if (error) {
    log_error(error, "key_agreement_prologue_xx");
    goto exit;
  }

  // 2. Generate an ephemeral 25519 keypair for this handshake and set it to e
  error = ockam_vault_secret_generate(xx->vault, &xx->e_secret, &secret_attributes);
  if (error) {
    log_error(error, "key_agreement_prologue_xx");
    goto exit;
  }

  error = ockam_vault_secret_publickey_get(xx->vault, &xx->e_secret, xx->e, sizeof(xx->e), &key_size);
  if (error) {
    log_error(error, "key_agreement_prologue_xx");
    goto exit;
  }

  // 3. Set k to empty, Set n to 0
  xx->nonce = 0;
  memset(xx->k, 0, KEY_SIZE);

  // 4. Set h and ck to 'Noise_XX_25519_AESGCM_SHA256'
  memset(xx->h, 0, SHA256_SIZE);
  memcpy(xx->h, PROTOCOL_NAME, PROTOCOL_NAME_SIZE);
  memset(ck, 0, KEY_SIZE);
  memcpy(ck, PROTOCOL_NAME, PROTOCOL_NAME_SIZE);
  secret_attributes.type = OCKAM_VAULT_SECRET_TYPE_BUFFER;
  error                  = ockam_vault_secret_import(xx->vault, &xx->ck_secret, &secret_attributes, ck, KEY_SIZE);
  if (error) goto exit;

  // 5. h = SHA256(h || prologue),
  // prologue is empty
  mix_hash(xx, NULL, 0);

exit:
  if (error) log_error(error, __func__);
  return error;
}

/*------------------------------------------------------------------------------------------------------*
 *          UTILITY FUNCTIONS
 *------------------------------------------------------------------------------------------------------*/
void print_uint8_str(uint8_t* p, uint16_t size, char* msg)
{
  printf("\n%s %d bytes: \n", msg, size);
  for (int i = 0; i < size; ++i) printf("%0.2x", *p++);
  printf("\n");
}

ockam_error_t hkdf_dh(key_establishment_xx* xx,
                      ockam_vault_secret_t* salt,
                      ockam_vault_secret_t* privatekey,
                      uint8_t*              peer_publickey,
                      size_t                peer_publickey_length,
                      ockam_vault_secret_t* secret1,
                      ockam_vault_secret_t* secret2)
{
  ockam_error_t        error = OCKAM_ERROR_NONE;
  ockam_vault_secret_t shared_secret;
  ockam_vault_secret_t generated_secrets[2];

  // Compute shared secret
  // error = ockam_vault_ecdh( dh_key_type, dh2, dh2_size, secret, KEY_SIZE );
  error = ockam_vault_ecdh(xx->vault, privatekey, peer_publickey, peer_publickey_length, &shared_secret);
  if (OCKAM_ERROR_NONE != error) {
    log_error(error, "failed ockam_vault_ecdh in responder_m2_send");
    goto exit;
  }

  // ck, k = HKDF( ck, shared_secret )
  error = ockam_vault_hkdf_sha256(xx->vault, salt, &shared_secret, 2, generated_secrets);
  if (OCKAM_ERROR_NONE != error) {
    log_error(error, "failed ockam_vault_hkdf_sha256 in hkdf_dh");
    goto exit;
  }

  memcpy(secret1, &generated_secrets[0], sizeof(ockam_vault_secret_t));
  memcpy(secret2, &generated_secrets[1], sizeof(ockam_vault_secret_t));

exit:
  return error;
}

void string_to_hex(uint8_t* hexstring, uint8_t* val, size_t* p_bytes)
{
  const char* pos   = (char*) hexstring;
  uint32_t    bytes = 0;

  for (size_t count = 0; count < (strlen((char*) hexstring) / 2); count++) {
    sscanf(pos, "%2hhx", &val[count]);
    pos += 2;
    bytes += 1;
  }
  if (NULL != p_bytes) *p_bytes = bytes;
}

void mix_hash(key_establishment_xx* xx, uint8_t* p_bytes, uint16_t b_length)
{
  ockam_error_t error;
  uint8_t*      p_h = &xx->h[0];
  uint8_t       string[MAX_TRANSMIT_SIZE];
  uint8_t       hash[SHA256_SIZE];
  size_t        hash_length = 0;

  memset(&hash[0], 0, sizeof(hash));
  memset(&string[0], 0, sizeof(string));
  memcpy(&string[0], &p_h[0], SHA256_SIZE);
  memcpy(&string[SHA256_SIZE], p_bytes, b_length);
  error = ockam_vault_sha256(xx->vault, string, SHA256_SIZE + b_length, hash, SHA256_SIZE, &hash_length);
  if (error) log_error(error, "mix_hash");
  memcpy(p_h, hash, hash_length);

exit:
  return;
}

ockam_error_t make_vector(uint64_t nonce, uint8_t* vector)
{
  uint8_t* pv;
  uint8_t* pn = (uint8_t*) &nonce;

  memset(vector, 0, VECTOR_SIZE);
  pv = vector + 4;
  pn += 7;
  for (int i = 7; i >= 0; --i) { *pv++ = *pn--; }
  return OCKAM_ERROR_NONE;
}
