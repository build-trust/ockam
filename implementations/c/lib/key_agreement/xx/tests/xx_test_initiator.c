
#include <stdio.h>
#include <string.h>
#include <stdbool.h>

#include "ockam/error.h"
#include "ockam/key_agreement.h"
#include "../../xx/xx_local.h"
#include "ockam/memory.h"
#include "ockam/syslog.h"
#include "ockam/transport.h"
#include "ockam/vault.h"
#include "xx_test.h"

extern bool scripted_xx;

ockam_error_t xx_test_initiator_prologue(key_establishment_xx* xx)
{
  ockam_error_t                   error             = OCKAM_ERROR_NONE;
  ockam_vault_secret_attributes_t secret_attributes = { KEY_SIZE,
                                                        OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY,
                                                        OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT,
                                                        OCKAM_VAULT_SECRET_EPHEMERAL };
  uint8_t                         key[KEY_SIZE];
  size_t                          key_bytes;
  uint8_t                         ck[KEY_SIZE];

  // 1. Pick a static 25519 keypair for this handshake and set it to s

  string_to_hex((uint8_t*) INITIATOR_STATIC, key, &key_bytes);
  error = ockam_vault_secret_import(xx->vault, &xx->s_secret, &secret_attributes, key, key_bytes);
  if (error) {
    log_error(error, "xx_test_initiator_prologue");
    goto exit;
  }

  error = ockam_vault_secret_publickey_get(xx->vault, &xx->s_secret, xx->s, KEY_SIZE, &key_bytes);
  if (error) {
    log_error(error, "xx_test_initiator_prologue");
    goto exit;
  }

  // 2. Generate an ephemeral 25519 keypair for this handshake and set it to e

  string_to_hex((uint8_t*) INITIATOR_EPH, key, &key_bytes);
  secret_attributes.persistence = OCKAM_VAULT_SECRET_EPHEMERAL;
  error = ockam_vault_secret_import(xx->vault, &xx->e_secret, &secret_attributes, key, key_bytes);
  if (error) {
    log_error(error, "xx_test_initiator_prologue");
    goto exit;
  }

  error = ockam_vault_secret_publickey_get(xx->vault, &xx->e_secret, xx->e, KEY_SIZE, &key_bytes);
  if (error) {
    log_error(error, "xx_test_initiator_prologue");
    goto exit;
  }

  // Nonce to 0, k to empty
  xx->nonce = 0;
  memset(xx->k, 0, sizeof(xx->k));

  // 4. Set h and ck to 'Noise_XX_25519_AESGCM_SHA256'
  memset(xx->h, 0, SHA256_SIZE);
  memcpy(xx->h, PROTOCOL_NAME, PROTOCOL_NAME_SIZE);
  memset(ck, 0, KEY_SIZE);
  memcpy(ck, PROTOCOL_NAME, PROTOCOL_NAME_SIZE);
  secret_attributes.type = OCKAM_VAULT_SECRET_TYPE_BUFFER;
  error                  = ockam_vault_secret_import(xx->vault, &xx->ck_secret, &secret_attributes, ck, KEY_SIZE);
  if (error) {
    log_error(error, "xx_test_responder_prologue");
    goto exit;
  }

  // 5. h = SHA256(h || prologue),
  // prologue is empty
  mix_hash(xx, NULL, 0);

exit:
  return error;
}

ockam_error_t test_initiator_handshake(key_establishment_xx* xx)
{
  ockam_error_t error = TRANSPORT_ERROR_NONE;
  uint8_t       sendBuffer[MAX_TRANSMIT_SIZE];
  uint8_t       recv_buffer[MAX_TRANSMIT_SIZE];
  size_t        bytesReceived = 0;
  size_t        transmit_size = 0;
  uint8_t       compare[1024];
  size_t        compare_bytes;

  /* Prologue initializes keys and handshake parameters */
  error = xx_test_initiator_prologue(xx);
  if (error) {
    log_error(error, "TestInitiatorPrologue");
    goto exit;
  }

  // Step 1 generate message
  error = xx_initiator_m1_make(xx, sendBuffer, MAX_TRANSMIT_SIZE, &transmit_size);
  if (TRANSPORT_ERROR_NONE != error) {
    log_error(error, "initiator_step_1 failed");
    goto exit;
  }

  // Verify
  string_to_hex((uint8_t*) MSG_1_CIPHERTEXT, compare, &compare_bytes);
  if (0 != memcmp(sendBuffer, compare, compare_bytes)) {
    error = kXXKeyAgreementTestFailed;
    log_error(error, "Test failed on msg 0\n");
    goto exit;
  }

  // Step 1 send message
  error = ockam_write(xx->p_writer, sendBuffer, transmit_size);
  if (TRANSPORT_ERROR_NONE != error) {
    log_error(error, "ockam_SendBlocking after initiator_step_1 failed");
    goto exit;
  }

  // Msg 2 receive
  error = ockam_read(xx->p_reader, recv_buffer, sizeof(recv_buffer), &bytesReceived);
  if (TRANSPORT_ERROR_NONE != error) {
    log_error(error, "ockam_ReceiveBlocking failed on msg 2");
    goto exit;
  }

  // Msg 2 process
  error = xx_initiator_m2_process(xx, recv_buffer, bytesReceived);
  if (TRANSPORT_ERROR_NONE != error) {
    log_error(error, "ockam_ReceiveBlocking failed on msg 2");
    goto exit;
  }

  // Msg 3 make
  error = xx_initiator_m3_make(xx, sendBuffer, &transmit_size);
  if (TRANSPORT_ERROR_NONE != error) {
    log_error(error, "initiator_m3_make failed");
    goto exit;
  }

  /* Msg 3 verify */
  string_to_hex((uint8_t*) MSG_3_CIPHERTEXT, compare, &compare_bytes);
  if (0 != memcmp(compare, sendBuffer, transmit_size)) {
    error = kXXKeyAgreementTestFailed;
    log_error(error, "-------Msg 3 verify failed");
    goto exit;
  }

  // Msg 3 send
  error = ockam_write(xx->p_writer, sendBuffer, transmit_size);
  if (TRANSPORT_ERROR_NONE != error) {
    log_error(error, "ockam_SendBlocking failed on msg 3");
    goto exit;
  }

  error = xx_initiator_epilogue(xx);
  if (TRANSPORT_ERROR_NONE != error) {
    log_error(error, "initiator_epilogue failed");
    goto exit;
  }

exit:
  return error;
}

ockam_error_t establish_initiator_transport(ockam_transport_t** transport,
                                            ockam_ip_address_t* ip_address,
                                            ockam_reader_t**    pp_reader,
                                            ockam_writer_t**    pp_writer)
{
  ockam_error_t                           error = kXXKeyAgreementFailed;
  ockam_transport_tcp_socket_attributes_t tcp_attrs;
  memset(&tcp_attrs, 0, sizeof(tcp_attrs));

  error = ockam_transport_socket_tcp_init(transport, &tcp_attrs);
  if (error) {
    log_error(error, "establish_initiator_transport");
    goto exit;
  }

  error = ockam_transport_connect(*transport, pp_reader, pp_writer, ip_address);
  if (error) {
    log_error(error, "establish_initiator_transport");
    goto exit;
  }

  error = TRANSPORT_ERROR_NONE;
exit:
  return error;
}

ockam_error_t xx_test_initiator(ockam_vault_t* vault, ockam_ip_address_t* ip_address)
{
  ockam_transport_t* transport = NULL;

  ockam_error_t        error = kXXKeyAgreementFailed;
  key_establishment_xx handshake;
  uint8_t              sendBuffer[MAX_TRANSMIT_SIZE];
  uint8_t              recv_buffer[MAX_TRANSMIT_SIZE];
  size_t               bytes_received = 0;
  size_t               transmit_size  = 0;
  uint8_t              test[TEST_MSG_CIPHER_SIZE];
  size_t               test_bytes;
  uint8_t              test_responder[TEST_MSG_CIPHER_SIZE];

  memset(&handshake, 0, sizeof(handshake));
  handshake.vault = vault;

  error = establish_initiator_transport(&transport, ip_address, &handshake.p_reader, &handshake.p_writer);
  if (error) {
    log_error(error, "Failed to establish transportCtx with responder");
    goto exit;
  }

  if (scripted_xx) {
    error = test_initiator_handshake(&handshake);
  } else {
    error = ockam_key_establish_initiator_xx(&handshake);
  }
  if (TRANSPORT_ERROR_NONE != error) {
    log_error(error, "ockam_initiator_handshake");
    goto exit;
  }

  /*-------------------------------------------------------------------------
   * Receive the test message
   *-----------------------------------------------------------------------*/
  error = ockam_read(handshake.p_reader, recv_buffer, sizeof(recv_buffer), &bytes_received);
  if (TRANSPORT_ERROR_NONE != error) {
    log_error(error, "ockam_ReceiveBlocking failed on test message");
    goto exit;
  }

  /*-------------------------------------------------------------------------
   * Confirm the test message
   *-----------------------------------------------------------------------*/
  error = xx_decrypt(&handshake, test, TEST_MSG_CIPHER_SIZE, recv_buffer, bytes_received, &test_bytes);
  if (TRANSPORT_ERROR_NONE != error) {
    log_error(error, "xx_decrypt failed on test msg");
    goto exit;
  }
  if (scripted_xx) {
    string_to_hex((uint8_t*) TEST_MSG_RESPONDER, test_responder, NULL);
    if (0 != memcmp((void*) test, test_responder, TEST_MSG_BYTE_SIZE)) { error = kXXKeyAgreementTestFailed; }
  } else {
    if (0 != memcmp(ACK, test, ACK_SIZE)) { error = kXXKeyAgreementTestFailed; }
  }
  if (OCKAM_ERROR_NONE != error) {
    log_error(error, "Received bad epilogue message");
    goto exit;
  }
  /*-------------------------------------------------------------------------
   * Make the test message
   *-----------------------------------------------------------------------*/
  if (scripted_xx) {
    string_to_hex((uint8_t*) TEST_MSG_INITIATOR, test, &test_bytes);
    error = xx_encrypt(&handshake, test, test_bytes, sendBuffer, sizeof(sendBuffer), &transmit_size);
  } else {
    error = xx_encrypt(&handshake, (uint8_t*) OK, OK_SIZE, sendBuffer, sizeof(sendBuffer), &transmit_size);
  }
  if (TRANSPORT_ERROR_NONE != error) {
    log_error(error, "initiator_encrypt failed on test message");
    goto exit;
  }

  /*-------------------------------------------------------------------------
   * Confirm the test message
   *-----------------------------------------------------------------------*/
  if (scripted_xx) {
    string_to_hex((uint8_t*) MSG_5_CIPHERTEXT, test, &test_bytes);
    if (0 != memcmp(test, sendBuffer, transmit_size)) {
      error = kXXKeyAgreementTestFailed;
      log_error(error, "Msg 5 failed");
      goto exit;
    }
  }

  /*-------------------------------------------------------------------------
   * Send the test message
   *-----------------------------------------------------------------------*/
  error = ockam_write(handshake.p_writer, sendBuffer, transmit_size);
  if (TRANSPORT_ERROR_NONE != error) {
    log_error(error, "ockam_SendBlocking failed on test message");
    goto exit;
  }

exit:
  if (NULL != transport) ockam_transport_deinit(transport);
  return error;
}
