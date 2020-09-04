#include <stdio.h>
#include <stdbool.h>
#include <string.h>
#include <unistd.h>

#include "ockam/error.h"
#include "ockam/key_agreement/xx.h"
#include "ockam/key_agreement/xx_local.h"
#include "ockam/key_agreement.h"
#include "ockam/memory.h"
#include "ockam/log.h"
#include "ockam/transport.h"
#include "ockam/transport/socket_udp.h"
#include "ockam/vault.h"
#include "xx_test.h"

extern bool scripted_xx;

ockam_error_t xx_test_responder_prologue(xx_key_exchange_ctx_t* xx)
{
  ockam_error_t                   error             = ockam_key_agreement_xx_error_none;
  ockam_vault_secret_attributes_t secret_attributes = { PRIVATE_KEY_SIZE,
                                                        OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY,
                                                        OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT,
                                                        OCKAM_VAULT_SECRET_EPHEMERAL };
  uint8_t                         key[PRIVATE_KEY_SIZE];
  size_t                          key_bytes;
  uint8_t                         ck[SYMMETRIC_KEY_SIZE];

  // 1. Pick a static 25519 keypair for this xx and set it to s
  string_to_hex((uint8_t*) RESPONDER_STATIC, key, &key_bytes);
  error = ockam_vault_secret_import(xx->vault, &xx->s_secret, &secret_attributes, key, key_bytes);
  if (ockam_error_has_error(&error)) { goto exit; }

  error = ockam_vault_secret_publickey_get(xx->vault, &xx->s_secret, xx->s, P256_PUBLIC_KEY_SIZE, &key_bytes);
  if (ockam_error_has_error(&error)) { goto exit; }

  // 2. Generate an ephemeral 25519 keypair for this xx and set it to e
  string_to_hex((uint8_t*) RESPONDER_EPH, key, &key_bytes);
  secret_attributes.persistence = OCKAM_VAULT_SECRET_EPHEMERAL;
  error = ockam_vault_secret_import(xx->vault, &xx->e_secret, &secret_attributes, key, key_bytes);
  if (ockam_error_has_error(&error)) { goto exit; }

  error = ockam_vault_secret_publickey_get(xx->vault, &xx->e_secret, xx->e, P256_PUBLIC_KEY_SIZE, &key_bytes);
  if (ockam_error_has_error(&error)) { goto exit; }

  // Nonce to 0, k to empty
  xx->nonce = 0;

  // 4. Set h and ck to 'Noise_XX_25519_AESGCM_SHA256'
  memset(xx->h, 0, SHA256_SIZE);
  memcpy(xx->h, PROTOCOL_NAME, PROTOCOL_NAME_SIZE);
  memset(ck, 0, SYMMETRIC_KEY_SIZE);
  // FIXME
  memcpy(ck, PROTOCOL_NAME, PROTOCOL_NAME_SIZE);
  secret_attributes.type = OCKAM_VAULT_SECRET_TYPE_BUFFER;
  error                  = ockam_vault_secret_import(xx->vault, &xx->ck_secret, &secret_attributes, ck, SYMMETRIC_KEY_SIZE);
  if (ockam_error_has_error(&error)) { goto exit; }

  // 5. h = SHA256(h || prologue),
  // prologue is empty
  mix_hash(xx, NULL, 0);

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t test_responder_handshake(
  ockam_key_t* key, ockam_memory_t* memory, ockam_vault_t* vault, ockam_reader_t* reader, ockam_writer_t* writer)
{
  ockam_error_t   error = ockam_key_agreement_xx_error_none;
  uint8_t         write_buffer[MAX_XX_TRANSMIT_SIZE];
  uint8_t         read_buffer[MAX_XX_TRANSMIT_SIZE];
  size_t          transmit_size  = 0;
  size_t          bytes_received = 0;
  uint8_t         compare[1024];
  size_t          compare_bytes;
  ockam_xx_key_t* xx = NULL;

  xx         = (ockam_xx_key_t*) key->context;
  xx->reader = reader;
  xx->writer = writer;

  /* Prologue initializes keys and xx parameters */
  error = xx_test_responder_prologue(xx->exchange);
  if (ockam_error_has_error(&error)) goto exit;

  /* Msg 1 receive */
  do {
    error = ockam_read(xx->reader, read_buffer, MAX_XX_TRANSMIT_SIZE, &bytes_received);
    if (ockam_error_has_error(&error)) {
      if (!(error.code == OCKAM_TRANSPORT_INTERFACE_ERROR_NO_DATA
            && error.domain == OCKAM_TRANSPORT_INTERFACE_ERROR_DOMAIN)) goto exit;
      usleep(500);
    }
  } while (ockam_error_has_error(&error));
  /* Msg 1 process */
  error = xx_responder_m1_process(xx, read_buffer);
  if (ockam_error_has_error(&error)) goto exit;

  /* Msg 2 make */
  error = xx_responder_m2_make(xx, write_buffer, sizeof(write_buffer), &transmit_size);
  if (ockam_error_has_error(&error)) goto exit;

  /* Msg 2 verify */
  string_to_hex((uint8_t*) MSG_2_CIPHERTEXT, compare, &compare_bytes);
  if (0 != memcmp(&write_buffer, compare, compare_bytes)) {
    error.code = -1;
    goto exit;
  }

  /* Msg 2 send */
  error = ockam_write(xx->writer, write_buffer, transmit_size);
  if (ockam_error_has_error(&error)) goto exit;

  /* Msg 3 receive */
  do {
    error = ockam_read(xx->reader, read_buffer, MAX_XX_TRANSMIT_SIZE, &bytes_received);
    if (ockam_error_has_error(&error)) {
      if (!(error.code == OCKAM_TRANSPORT_INTERFACE_ERROR_NO_DATA
            && error.domain == OCKAM_TRANSPORT_INTERFACE_ERROR_DOMAIN)) goto exit;
      usleep(500);
    }
  } while (ockam_error_has_error(&error));
  /* Msg 3 process */
  error = xx_responder_m3_process(xx, read_buffer);
  if (ockam_error_has_error(&error)) goto exit;

  /* Epilogue */
  error = xx_responder_epilogue(key);
  if (ockam_error_has_error(&error)) { goto exit; }

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t establish_responder_connection(ockam_transport_t*  p_transport,
                                             ockam_memory_t*     p_memory,
                                             ockam_ip_address_t* p_address,
                                             ockam_reader_t**    pp_reader,
                                             ockam_writer_t**    pp_writer)
{
  ockam_error_t                       error = ockam_key_agreement_xx_error_none;
  ockam_transport_socket_attributes_t transport_attributes;

  memcpy(&transport_attributes.local_address, p_address, sizeof(ockam_ip_address_t));
  transport_attributes.p_memory = p_memory;
  error                         = ockam_transport_socket_udp_init(p_transport, &transport_attributes);
  if (ockam_error_has_error(&error)) goto exit;

  // Wait for a connection
  error = ockam_transport_accept(p_transport, pp_reader, pp_writer, NULL);
  if (ockam_error_has_error(&error)) { goto exit; }

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t run_responder_exchange(ockam_key_t* key, struct ockam_reader_t* reader, struct ockam_writer_t* writer)
{
  ockam_error_t error = ockam_key_agreement_xx_error_none;
  uint8_t       message[MAX_XX_TRANSMIT_SIZE];
  size_t        message_length = 0;

  /* Msg 1 receive */
  do {
    error = ockam_read(reader, message, sizeof(message), &message_length);
    if (ockam_error_has_error(&error)) {
      if (!(error.code == OCKAM_TRANSPORT_INTERFACE_ERROR_NO_DATA
            && error.domain == OCKAM_TRANSPORT_INTERFACE_ERROR_DOMAIN)) goto exit;
      usleep(500 * 1000);
    }
  } while (ockam_error_has_error(&error));
  /* Msg 1 process */
  error = ockam_key_m1_process(key, message);
  if (ockam_error_has_error(&error)) goto exit;

  /* Msg 2 make */
  error = ockam_key_m2_make(key, message, sizeof(message), &message_length);
  if (ockam_error_has_error(&error)) goto exit;

  /* Msg 2 send */
  error = ockam_write(writer, message, message_length);
  if (ockam_error_has_error(&error)) goto exit;

  /* Msg 3 receive */
  do {
    error = ockam_read(reader, message, sizeof(message), &message_length);
    if (ockam_error_has_error(&error)) {
      if (!(error.code == OCKAM_TRANSPORT_INTERFACE_ERROR_NO_DATA
            && error.domain == OCKAM_TRANSPORT_INTERFACE_ERROR_DOMAIN)) goto exit;
      usleep(500);
    }
  } while (ockam_error_has_error(&error));
  /* Msg 3 process */
  error = ockam_key_m3_process(key, message);
  if (ockam_error_has_error(&error)) goto exit;

  /* Epilogue */
  error = ockam_responder_epilogue(key);
  if (ockam_error_has_error(&error)) goto exit;
  printf("Responder secure\n");
exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);

  return error;
}

extern ockam_memory_t*            gp_ockam_key_memory;
extern ockam_key_dispatch_table_t xx_key_dispatch;

ockam_error_t test_responder_initialize(ockam_key_t* key, ockam_memory_t* memory, ockam_vault_t* vault)
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
  error = xx_test_responder_prologue(p_xx_key->exchange);
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

ockam_error_t xx_test_responder(ockam_vault_t* p_vault, ockam_memory_t* p_memory, ockam_ip_address_t* ip_address)
{
  ockam_transport_t transport = { 0 };
  ockam_error_t     error     = ockam_key_agreement_xx_error_none;

  uint8_t         write_buffer[MAX_XX_TRANSMIT_SIZE];
  uint8_t         read_buffer[MAX_XX_TRANSMIT_SIZE];
  size_t          transmit_size = 0;
  uint8_t         test[16];
  size_t          test_size;
  uint8_t         test_initiator[TEST_MSG_BYTE_SIZE];
  uint8_t         comp[2048];
  size_t          comp_size;
  ockam_key_t     key;
  ockam_reader_t* p_reader;
  ockam_writer_t* p_writer;

  /*-------------------------------------------------------------------------
   * Establish transport connection with responder
   *-----------------------------------------------------------------------*/
  error = establish_responder_connection(&transport, p_memory, ip_address, &p_reader, &p_writer);
  if (ockam_error_has_error(&error)) goto exit;

  ockam_log_info("Responder key agreement initialized");

  /*-------------------------------------------------------------------------
   * Perform the secret xx
   * If successful, encrypt/decrypt keys will be established
   *-----------------------------------------------------------------------*/

  if (scripted_xx) {
    error = test_responder_initialize(&key, p_memory, p_vault);
    if (ockam_error_has_error(&error)) goto exit;
    error = test_responder_handshake(&key, p_memory, p_vault, p_reader, p_writer);
    if (ockam_error_has_error(&error)) goto exit;
  } else {
    error = ockam_xx_key_initialize(&key, p_memory, p_vault);
    if (ockam_error_has_error(&error)) goto exit;
    error = run_responder_exchange(&key, p_reader, p_writer);
  }
  if (ockam_error_has_error(&error)) { goto exit; }

  ockam_log_info("Responder key agreement responded");

  /*-------------------------------------------------------------------------
   * Verify secure channel by sending and receiving a known message
   *-----------------------------------------------------------------------*/

  if (scripted_xx) {
    /* Convert string to hex bytes and encrypt */
    string_to_hex((uint8_t*) TEST_MSG_RESPONDER, test, &test_size);
    error = ockam_key_encrypt(&key, test, test_size, write_buffer, sizeof(write_buffer), &transmit_size);
    if (ockam_error_has_error(&error)) goto exit;
    /* Verify test message ciphertext */
    string_to_hex((uint8_t*) MSG_4_CIPHERTEXT, comp, &comp_size);
    if (0 != memcmp(comp, write_buffer, transmit_size)) {
      error.code = -1;
      goto exit;
    }
  } else {
    error = ockam_key_encrypt(&key, (uint8_t*) ACK_TEXT, ACK_SIZE, write_buffer, sizeof(write_buffer), &transmit_size);
    if (ockam_error_has_error(&error)) {
      ockam_log_error("%s: %d", error.domain, error.code);
      goto exit;
    }
  }

  ockam_log_info("Responder test message encrypted");

  /* Send test message */
  error = ockam_write(p_writer, write_buffer, transmit_size);
  if (ockam_error_has_error(&error)) goto exit;

  ockam_log_info("Responder test message sent");

  /* Receive test message  */
  memset(read_buffer, 0, sizeof(read_buffer));
  do {
    error = ockam_read(p_reader, read_buffer, MAX_XX_TRANSMIT_SIZE, &transmit_size);
    if (ockam_error_has_error(&error)) {
      if (!(error.code == OCKAM_TRANSPORT_INTERFACE_ERROR_NO_DATA
            && error.domain == OCKAM_TRANSPORT_INTERFACE_ERROR_DOMAIN)) goto exit;
      usleep(50000);
    }
  } while (ockam_error_has_error(&error));

  ockam_log_info("Responder test message receiver");

  /* Decrypt test message */

  error = ockam_key_decrypt(&key, test, TEST_MSG_BYTE_SIZE, read_buffer, transmit_size, &test_size);
  if (ockam_error_has_error(&error)) goto exit;

  ockam_log_info("Responder test message decrypted");

  /* Verify test message */
  if (scripted_xx) {
    string_to_hex((uint8_t*) TEST_MSG_INITIATOR, test_initiator, NULL);
    if (0 != memcmp((void*) test, test_initiator, TEST_MSG_BYTE_SIZE)) {
      error.code = -1;
      goto exit;
    }
  } else {
    if (0 != memcmp(OK, test, OK_SIZE)) {
      error.code = -1;
      goto exit;
    }
  }
exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  ockam_transport_deinit(&transport);
  printf("Responder test ended with error %d\n", error.code);
  return error;
}
