#ifndef HANDSHAKE_LOCAL_H
#define HANDSHAKE_LOCAL_H
#include <stdlib.h>

#include "ockam/error.h"
#include "ockam/io.h"
#include "ockam/transport.h"
#include "ockam/vault.h"
#include "key_agreement/key_impl.h"
#include "ockam/key_agreement.h"

#define PROTOCOL_NAME        "Noise_XX_25519_AESGCM_SHA256"
#define PROTOCOL_NAME_SIZE   28
#define MAX_XX_TRANSMIT_SIZE 1028
#define TAG_SIZE             16
#define VECTOR_SIZE          12

#define DEFAULT_IP_ADDRESS "127.0.0.1"
#define DEFAULT_LISTEN_PORT 4000

struct ockam_xx_key {
  ockam_vault_secret_t encrypt_secret;
  ockam_vault_secret_t decrypt_secret;
  uint16_t             encrypt_nonce;
  uint16_t             decrypt_nonce;
  ockam_vault_t*       p_vault;
  ockam_reader_t*      p_reader;
  ockam_writer_t*      p_writer;
};

typedef struct ockam_xx_key ockam_xx_key_t;

typedef struct {
  uint16_t             nonce;
  uint8_t              s[KEY_SIZE];
  ockam_vault_secret_t s_secret;
  uint8_t              rs[KEY_SIZE];
  uint8_t              e[KEY_SIZE];
  ockam_vault_secret_t e_secret;
  uint8_t              re[KEY_SIZE];
  uint8_t              k[KEY_SIZE];
  ockam_vault_secret_t k_secret;
  uint8_t              ck[KEY_SIZE];
  ockam_vault_secret_t ck_secret;
  uint8_t              h[SHA256_SIZE];
  ockam_vault_t*       vault;
} key_establishment_xx;

void print_uint8_str(uint8_t* p, uint16_t size, char* msg);
void string_to_hex(uint8_t* hexstring, uint8_t* val, size_t* p_bytes);
void mix_hash(key_establishment_xx* p_handshake, uint8_t* p_bytes, uint16_t b_length);

ockam_error_t ockam_key_establish_initiator_xx(void* p_context);
ockam_error_t ockam_key_establish_responder_xx(void* p_context);

ockam_error_t key_agreement_prologue_xx(key_establishment_xx* xx);

ockam_error_t xx_responder_m1_process(key_establishment_xx* p_h, uint8_t* p_m1, size_t m1_size);
ockam_error_t xx_responder_m2_make(key_establishment_xx* p_h, uint8_t* p_msg, size_t msg_size, size_t* p_bytesWritten);
ockam_error_t xx_responder_m3_process(key_establishment_xx* p_h, uint8_t* p_m3, size_t m3_size);
ockam_error_t xx_responder_epilogue(key_establishment_xx* p_h, ockam_xx_key_t* p_key);
ockam_error_t
              xx_initiator_m1_make(key_establishment_xx* p_h, uint8_t* p_sendBuffer, size_t buffer_length, size_t* p_transmit_size);
ockam_error_t xx_initiator_m2_process(key_establishment_xx* p_h, uint8_t* p_recv, size_t recv_size);
ockam_error_t xx_initiator_m3_make(key_establishment_xx* p_h, uint8_t* p_msg, size_t* p_msg_size);
ockam_error_t xx_initiator_epilogue(key_establishment_xx* p_h, ockam_xx_key_t* p_key);
ockam_error_t
              xx_encrypt(void* p_context, uint8_t* payload, size_t payload_size, uint8_t* msg, size_t msg_length, size_t* msg_size);
ockam_error_t xx_decrypt(
  void* p_context, uint8_t* payload, size_t payload_size, uint8_t* msg, size_t msg_length, size_t* payload_bytes);
ockam_error_t xx_key_deinit(void* p_context);
ockam_error_t make_vector(uint64_t nonce, uint8_t* p_vector);
ockam_error_t hkdf_dh(key_establishment_xx* xx,
                      ockam_vault_secret_t* salt,
                      ockam_vault_secret_t* privatekey,
                      uint8_t*              peer_publickey,
                      size_t                peer_publickey_length,
                      ockam_vault_secret_t* secret1,
                      ockam_vault_secret_t* secret2);
#endif
