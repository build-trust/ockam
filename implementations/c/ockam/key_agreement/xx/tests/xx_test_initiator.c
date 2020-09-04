
#include <stdio.h>
#include <string.h>
#include <stdbool.h>
#include <unistd.h>

#include "ockam/error.h"
#include "ockam/key_agreement.h"
#include "ockam/key_agreement/xx.h"
#include "ockam/key_agreement/xx_local.h"
#include "ockam/memory.h"
#include "ockam/log.h"
#include "ockam/transport.h"
#include "ockam/transport/socket_udp.h"
#include "ockam/vault.h"
#include "xx_test.h"

extern bool scripted_xx;

ockam_error_t xx_test_initiator_prologue(xx_key_exchange_ctx_t* xx)
{
  ockam_error_t                   error             = ockam_key_agreement_xx_error_none;
  ockam_vault_secret_attributes_t secret_attributes = { PRIVATE_KEY_SIZE,
                                                        OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY,
                                                        OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT,
                                                        OCKAM_VAULT_SECRET_EPHEMERAL };
  uint8_t                         key[PRIVATE_KEY_SIZE];
  size_t                          key_bytes;
  uint8_t                         ck[SYMMETRIC_KEY_SIZE];

  // 1. Pick a static 25519 keypair for this handshake and set it to s

  string_to_hex((uint8_t*) INITIATOR_STATIC, key, &key_bytes);
  error = ockam_vault_secret_import(xx->vault, &xx->s_secret, &secret_attributes, key, key_bytes);
  if (ockam_error_has_error(&error)) goto exit;

  error = ockam_vault_secret_publickey_get(xx->vault, &xx->s_secret, xx->s, P256_PUBLIC_KEY_SIZE, &key_bytes);
  if (ockam_error_has_error(&error)) goto exit;

  // 2. Generate an ephemeral 25519 keypair for this handshake and set it to e

  string_to_hex((uint8_t*) INITIATOR_EPH, key, &key_bytes);
  secret_attributes.persistence = OCKAM_VAULT_SECRET_EPHEMERAL;
  error = ockam_vault_secret_import(xx->vault, &xx->e_secret, &secret_attributes, key, key_bytes);
  if (ockam_error_has_error(&error)) goto exit;

  error = ockam_vault_secret_publickey_get(xx->vault, &xx->e_secret, xx->e, P256_PUBLIC_KEY_SIZE, &key_bytes);
  if (ockam_error_has_error(&error)) goto exit;

  // Nonce to 0, k to empty
  xx->nonce = 0;

  // 4. Set h and ck to 'Noise_XX_25519_AESGCM_SHA256'
  memset(xx->h, 0, SHA256_SIZE);
  memcpy(xx->h, PROTOCOL_NAME, PROTOCOL_NAME_SIZE);
  memset(ck, 0, SYMMETRIC_KEY_SIZE);
  memcpy(ck, PROTOCOL_NAME, PROTOCOL_NAME_SIZE);
  secret_attributes.type = OCKAM_VAULT_SECRET_TYPE_BUFFER;
  error                  = ockam_vault_secret_import(xx->vault, &xx->ck_secret, &secret_attributes, ck, SYMMETRIC_KEY_SIZE);
  if (ockam_error_has_error(&error)) goto exit;

  // 5. h = SHA256(h || prologue),
  // prologue is empty
  mix_hash(xx, NULL, 0);

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

extern ockam_memory_t*            gp_ockam_key_memory;
extern ockam_key_dispatch_table_t xx_key_dispatch;

ockam_error_t test_initiator_initialize(ockam_key_t* key, ockam_memory_t* memory, ockam_vault_t* vault)
{
  ockam_error_t   error    = ockam_key_agreement_xx_error_none;
  ockam_xx_key_t* p_xx_key = NULL;

  if (!key || !vault || !memory) {
    error.code = -1;
    goto exit;
  }

  gp_ockam_key_memory = memory;

  key->dispatch = &xx_key_dispatch;
  ockam_memory_alloc_zeroed(memory, &key->context, sizeof(ockam_xx_key_t));

  p_xx_key = (ockam_xx_key_t*) key->context;
  error    = ockam_memory_alloc_zeroed(memory, (void**) &p_xx_key->exchange, sizeof(xx_key_exchange_ctx_t));
  if (ockam_error_has_error(&error)) goto exit;
  p_xx_key->exchange->vault = vault;

  p_xx_key->vault = vault;

  /* Prologue initializes keys and handshake parameters */
  error = xx_test_initiator_prologue(p_xx_key->exchange);
  if (ockam_error_has_error(&error)) goto exit;

exit:
  if (ockam_error_has_error(&error)) {
    ockam_log_error("%s: %d", error.domain, error.code);
    if (key) {
      if (key->context) ockam_memory_free(memory, key->context, 0);
    }
  }
  return error;
}

ockam_error_t test_initiator_handshake(
  ockam_key_t* key, ockam_memory_t* memory, ockam_vault_t* vault, ockam_reader_t* reader, ockam_writer_t* writer)
{
  ockam_error_t   error = ockam_key_agreement_xx_error_none;
  uint8_t         write_buffer[MAX_XX_TRANSMIT_SIZE];
  uint8_t         read_buffer[MAX_XX_TRANSMIT_SIZE];
  size_t          bytes_received = 0;
  size_t          transmit_size  = 0;
  uint8_t         compare[1024];
  size_t          compare_bytes;
  ockam_xx_key_t* xx = NULL;

  xx         = (ockam_xx_key_t*) key->context;
  xx->reader = reader;
  xx->writer = writer;

  // Step 1 generate message
  error = xx_initiator_m1_make(xx, write_buffer, MAX_XX_TRANSMIT_SIZE, &transmit_size);
  if (ockam_error_has_error(&error)) goto exit;

  // Verify
  string_to_hex((uint8_t*) MSG_1_CIPHERTEXT, compare, &compare_bytes);
  if (0 != memcmp(&write_buffer, compare, compare_bytes)) {
    error.code = -1;
    goto exit;
  }

  // Step 1 send message
  error = ockam_write(xx->writer, write_buffer, transmit_size);
  if (ockam_error_has_error(&error)) goto exit;

  // Msg 2 receive
  do {
    error = ockam_read(xx->reader, read_buffer, sizeof(read_buffer), &bytes_received);
    if (ockam_error_has_error(&error)) {
      if (!(error.code == OCKAM_TRANSPORT_INTERFACE_ERROR_NO_DATA
            && error.domain == OCKAM_TRANSPORT_INTERFACE_ERROR_DOMAIN)) goto exit;
      usleep(50000);
    }
  } while (ockam_error_has_error(&error));
  // Msg 2 process
  error = xx_initiator_m2_process(xx, read_buffer);
  if (ockam_error_has_error(&error)) goto exit;

  // Msg 3 make
  error = xx_initiator_m3_make(xx, write_buffer, MAX_XX_TRANSMIT_SIZE, &transmit_size);
  if (ockam_error_has_error(&error)) goto exit;

  /* Msg 3 verify */
  string_to_hex((uint8_t*) MSG_3_CIPHERTEXT, compare, &compare_bytes);
  if (0 != memcmp(compare, &write_buffer, transmit_size)) {
    error.code = -1;
    goto exit;
  }

  // Msg 3 send
  error = ockam_write(xx->writer, write_buffer, transmit_size);
  if (ockam_error_has_error(&error)) goto exit;

  error = xx_initiator_epilogue(key);
  if (ockam_error_has_error(&error)) goto exit;

  printf("Initiator secure\n");

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t establish_initiator_transport(ockam_transport_t*  p_transport,
                                            ockam_memory_t*     p_memory,
                                            ockam_ip_address_t* initiator_address,
                                            ockam_ip_address_t* responder_address,
                                            ockam_reader_t**    pp_reader,
                                            ockam_writer_t**    pp_writer)
{
  ockam_error_t error = ockam_key_agreement_xx_error_none;
  ockam_transport_socket_attributes_t tcp_attrs;
  memset(&tcp_attrs, 0, sizeof(tcp_attrs));
  tcp_attrs.p_memory = p_memory;

  memcpy(&tcp_attrs.local_address, initiator_address, sizeof(tcp_attrs.local_address));
  memcpy(&tcp_attrs.remote_address, responder_address, sizeof(tcp_attrs.remote_address));

  error = ockam_transport_socket_udp_init(p_transport, &tcp_attrs);
  if (ockam_error_has_error(&error)) { goto exit; }

  error = ockam_transport_connect(p_transport, pp_reader, pp_writer, 10, 1);
  if (ockam_error_has_error(&error)) goto exit;

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t run_initiator_exchange(ockam_key_t* key, struct ockam_reader_t* reader, struct ockam_writer_t* writer)
{
  ockam_error_t error = ockam_key_agreement_xx_error_none;
  uint8_t       message[MAX_XX_TRANSMIT_SIZE];
  size_t        message_length = 0;

  error = ockam_key_m1_make(key, message, sizeof(message), &message_length);
  if (ockam_error_has_error(&error)) goto exit;

  error = ockam_write(writer, message, message_length);
  if (ockam_error_has_error(&error)) goto exit;

  do {
    error = ockam_read(reader, message, sizeof(message), &message_length);
    if (ockam_error_has_error(&error)) {
      if (!(error.code == OCKAM_TRANSPORT_INTERFACE_ERROR_NO_DATA
            && error.domain == OCKAM_TRANSPORT_INTERFACE_ERROR_DOMAIN)) goto exit;
      usleep(500 * 1000);
    }
  } while (ockam_error_has_error(&error));
  error = ockam_key_m2_process(key, message);
  if (ockam_error_has_error(&error)) goto exit;

  error = ockam_key_m3_make(key, message, sizeof(message), &message_length);
  if (ockam_error_has_error(&error)) goto exit;

  error = ockam_write(writer, message, message_length);
  if (ockam_error_has_error(&error)) goto exit;

  error = ockam_initiator_epilogue(key);
  if (ockam_error_has_error(&error)) goto exit;
  printf("Initiator secure\n");

exit:
  return error;
}

ockam_error_t xx_test_initiator(ockam_vault_t*      p_vault,
                                ockam_memory_t*     p_memory,
                                ockam_ip_address_t* initiator_address,
                                ockam_ip_address_t* responder_address)
{
  ockam_transport_t transport = { 0 };

  ockam_error_t   error = ockam_key_agreement_xx_error_none;
  uint8_t         write_buffer[MAX_XX_TRANSMIT_SIZE];
  uint8_t         read_buffer[MAX_XX_TRANSMIT_SIZE];
  size_t          bytes_received = 0;
  size_t          transmit_size  = 0;
  uint8_t         test[TEST_MSG_CIPHER_SIZE];
  size_t          test_bytes;
  uint8_t         test_responder[TEST_MSG_CIPHER_SIZE];
  ockam_key_t     key = { 0 };
  ockam_reader_t* p_reader;
  ockam_writer_t* p_writer;

  error =
    establish_initiator_transport(&transport, p_memory, initiator_address, responder_address, &p_reader, &p_writer);
  if (ockam_error_has_error(&error)) goto exit;

  ockam_log_info("Initiator key agreement initialized");

  if (scripted_xx) {
    error = test_initiator_initialize(&key, p_memory, p_vault);
    if (ockam_error_has_error(&error)) goto exit;
    error = test_initiator_handshake(&key, p_memory, p_vault, p_reader, p_writer);
  } else {
    error = ockam_xx_key_initialize(&key, p_memory, p_vault);
    if (ockam_error_has_error(&error)) goto exit;
    error = run_initiator_exchange(&key, p_reader, p_writer);
  }
  if (ockam_error_has_error(&error)) goto exit;

    ockam_log_info("Initiator key agreement initiated");

  /*-------------------------------------------------------------------------
   * Receive the test message
   *-----------------------------------------------------------------------*/
  do {
    error = ockam_read(p_reader, read_buffer, sizeof(read_buffer), &bytes_received);
    if (ockam_error_has_error(&error)) {
      if (!(error.code == OCKAM_TRANSPORT_INTERFACE_ERROR_NO_DATA
            && error.domain == OCKAM_TRANSPORT_INTERFACE_ERROR_DOMAIN)) goto exit;
      usleep(500);
    }
  } while (ockam_error_has_error(&error));

    ockam_log_info("Initiator test message received");

  /*-------------------------------------------------------------------------
   * Confirm the test message
   *-----------------------------------------------------------------------*/
  error = ockam_key_decrypt(&key, test, TEST_MSG_CIPHER_SIZE, read_buffer, bytes_received, &test_bytes);
  if (ockam_error_has_error(&error)) goto exit;

    ockam_log_info("Initiator test message decrypted");

  if (scripted_xx) {
    string_to_hex((uint8_t*) TEST_MSG_RESPONDER, test_responder, NULL);
    if (0 != memcmp((void*) test, test_responder, TEST_MSG_BYTE_SIZE)) { error.code = -1; }
  } else {
    if (0 != memcmp(ACK_TEXT, test, ACK_SIZE)) { error.code = -1; }
  }
  if (ockam_error_has_error(&error)) goto exit;

  /*-------------------------------------------------------------------------
   * Make the test message
   *-----------------------------------------------------------------------*/
  if (scripted_xx) {
    string_to_hex((uint8_t*) TEST_MSG_INITIATOR, test, &test_bytes);
    error = ockam_key_encrypt(&key, test, test_bytes, write_buffer, sizeof(write_buffer), &transmit_size);
  } else {
    error = ockam_key_encrypt(&key, (uint8_t*) OK, OK_SIZE, write_buffer, sizeof(write_buffer), &transmit_size);
  }
  if (ockam_error_has_error(&error)) goto exit;

  /*-------------------------------------------------------------------------
   * Confirm the test message
   *-----------------------------------------------------------------------*/
  if (scripted_xx) {
    string_to_hex((uint8_t*) MSG_5_CIPHERTEXT, test, &test_bytes);
    if (0 != memcmp(test, write_buffer, transmit_size)) {
      error.code = -1;
      goto exit;
    }
  }

  /*-------------------------------------------------------------------------
   * Send the test message
   *-----------------------------------------------------------------------*/
  error = ockam_write(p_writer, write_buffer, transmit_size);
  if (ockam_error_has_error(&error)) { goto exit; }

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  ockam_transport_deinit(&transport);
  printf("Initiator test ended with error %d\n", error.code);
  return error;
}