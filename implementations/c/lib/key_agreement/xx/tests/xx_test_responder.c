/**
 ********************************************************************************************************
 * @file    xx_responder_test.c
 * @brief   Test program for the xx xx as per Noise XX 25519 AESGCM SHA256
 ********************************************************************************************************
 */
#include <stdio.h>
#include <stdbool.h>
#include <string.h>

#include "../../xx/xx_local.h"
#include "ockam/error.h"
#include "ockam/key_agreement.h"
#include "ockam/memory.h"
#include "ockam/syslog.h"
#include "ockam/transport.h"
#include "ockam/vault.h"
#include "xx_test.h"

extern bool scripted_xx;

/**
 ********************************************************************************************************
 *                                          TestResponderPrologue()
 ********************************************************************************************************
 *
 * Summary: This differs from the production xx_prologue in that it initiates
 *the xx with a known set of keys so that cipher results can be verified along
 *the way.
 *
 * @param xx [in/out] - pointer to xx struct
 * @return [out] - TRANSPORT_ERROR_NONE on success
 ********************************************************************************************************
 */
ockam_error_t xx_test_responder_prologue(key_establishment_xx* xx)
{
  ockam_error_t                   error             = OCKAM_ERROR_NONE;
  ockam_vault_secret_attributes_t secret_attributes = { KEY_SIZE,
                                                        OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY,
                                                        OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT,
                                                        OCKAM_VAULT_SECRET_EPHEMERAL };
  uint8_t                         key[KEY_SIZE];
  size_t                          key_bytes;
  uint8_t                         ck[KEY_SIZE];

  // 1. Pick a static 25519 keypair for this xx and set it to s
  string_to_hex((uint8_t*) RESPONDER_STATIC, key, &key_bytes);
  error = ockam_vault_secret_import(xx->vault, &xx->s_secret, &secret_attributes, key, key_bytes);
  if (error) {
    log_error(error, "xx_test_responder_prologue");
    goto exit;
  }

  error = ockam_vault_secret_publickey_get(xx->vault, &xx->s_secret, xx->s, KEY_SIZE, &key_bytes);
  if (error) {
    log_error(error, "xx_test_responder_prologue");
    goto exit;
  }

  // 2. Generate an ephemeral 25519 keypair for this xx and set it to e
  string_to_hex((uint8_t*) RESPONDER_EPH, key, &key_bytes);
  secret_attributes.persistence = OCKAM_VAULT_SECRET_EPHEMERAL;
  error = ockam_vault_secret_import(xx->vault, &xx->e_secret, &secret_attributes, key, key_bytes);
  if (error) {
    log_error(error, "xx_test_responder_prologue");
    goto exit;
  }

  error = ockam_vault_secret_publickey_get(xx->vault, &xx->e_secret, xx->e, KEY_SIZE, &key_bytes);
  if (error) {
    log_error(error, "xx_test_responder_prologue");
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

/**
 ********************************************************************************************************
 *                                          test_responder_handshake()
 ********************************************************************************************************
 *
 * Summary: Test the xx process by starting with predefined static and ephemeral
 *keys (generated in the prologue) and verifying intermediate results against
 *test data along the way
 *
 * @param connection [in] - initialized transport connection
 * @param xx [in/out] - pointer to xx structure
 * @return [out] - TRANSPORT_ERROR_NONE on success
 ********************************************************************************************************
 */
ockam_error_t test_responder_handshake(key_establishment_xx* xx)
{
  ockam_error_t error = OCKAM_ERROR_INTERFACE_KEYAGREEMENT;
  uint8_t       sendBuffer[MAX_TRANSMIT_SIZE];
  uint8_t       recv_buffer[MAX_TRANSMIT_SIZE];
  size_t        transmit_size = 0;
  size_t        bytesReceived = 0;
  uint8_t       compare[1024];
  size_t        compare_bytes;

  /* Prologue initializes keys and xx parameters */
  error = xx_test_responder_prologue(xx);
  if (TRANSPORT_ERROR_NONE != error) {
    log_error(error, "test_xx_prologue failed");
    goto exit;
  }
  /* Msg 1 receive */
  error = ockam_read(xx->p_reader, &recv_buffer[0], MAX_TRANSMIT_SIZE, &bytesReceived);
  if (error != TRANSPORT_ERROR_NONE) {
    log_error(error, "Read for msg 1 failed");
    goto exit;
  }

  /* Msg 1 process */
  error = xx_responder_m1_process(xx, recv_buffer, bytesReceived);
  if (error != TRANSPORT_ERROR_NONE) {
    log_error(error, "xx_responder_m1_process failed");
    goto exit;
  }

  /* Msg 2 make */
  error = xx_responder_m2_make(xx, sendBuffer, sizeof(sendBuffer), &transmit_size);
  if (error != TRANSPORT_ERROR_NONE) {
    log_error(error, "xx_responder_m2_make failed");
    goto exit;
  }

  /* Msg 2 verify */
  string_to_hex((uint8_t*) MSG_2_CIPHERTEXT, compare, &compare_bytes);
  if (0 != memcmp(sendBuffer, compare, compare_bytes)) {
    log_error(error, "Test failed on msg 2 make\n");
    goto exit;
  }

  /* Msg 2 send */
  error = ockam_write(xx->p_writer, sendBuffer, transmit_size);
  if (error != TRANSPORT_ERROR_NONE) {
    log_error(error, "responder_m2_send failed");
    goto exit;
  }

  /* Msg 3 receive */
  error = ockam_read(xx->p_reader, recv_buffer, MAX_TRANSMIT_SIZE, &bytesReceived);
  if (error != TRANSPORT_ERROR_NONE) {
    log_error(error, "ockam_ReceiveBlocking failed for msg 3");
    goto exit;
  }

  /* Msg 3 process */
  error = xx_responder_m3_process(xx, recv_buffer, bytesReceived);
  if (error != TRANSPORT_ERROR_NONE) {
    log_error(error, "responder_m3_process failed for msg 3");
    goto exit;
  }

  /* Epilogue */
  error = xx_responder_epilogue(xx);
  if (TRANSPORT_ERROR_NONE != error) {
    log_error(error, "Failed responder_epilogue");
    goto exit;
  }

exit:
  return error;
}

/**
 ********************************************************************************************************
 *                                   EstablishResponderConnection()
 ********************************************************************************************************
 *
 * Summary:
 *
 * @param listenerCtx
 * @param connectionCtx
 * @return
 */
ockam_error_t establish_responder_connection(ockam_transport_t** pp_transport,
                                             ockam_ip_address_t* p_address,
                                             ockam_reader_t**    pp_reader,
                                             ockam_writer_t**    pp_writer)
{
  ockam_error_t                           error = OCKAM_ERROR_INTERFACE_KEYAGREEMENT;
  ockam_transport_tcp_socket_attributes_t tcp_attributes;

  memcpy(&tcp_attributes.listen_address, p_address, sizeof(ockam_ip_address_t));
  error = ockam_transport_socket_tcp_init(pp_transport, &tcp_attributes);
  if (TRANSPORT_ERROR_NONE != error) {
    log_error(error, "failed PosixTcpInitialize");
    goto exit;
  }

  // Wait for a connection
  error = ockam_transport_accept(*pp_transport, pp_reader, pp_writer, NULL);
  if (error) {
    log_error(error, "establish_responder_connection");
    goto exit;
  }

  error = OCKAM_ERROR_NONE;

exit:
  return error;
}

ockam_error_t xx_test_responder(ockam_vault_t* vault, ockam_ip_address_t* ip_address)
{
  ockam_transport_t* transport = NULL;
  ockam_error_t      error     = OCKAM_ERROR_INTERFACE_KEYAGREEMENT;

  key_establishment_xx xx;
  uint8_t              sendBuffer[MAX_TRANSMIT_SIZE];
  uint8_t              recv_buffer[MAX_TRANSMIT_SIZE];
  size_t               transmit_size = 0;
  uint8_t              test[16];
  size_t               test_size;
  uint8_t              test_initiator[TEST_MSG_BYTE_SIZE];
  uint8_t              comp[2048];
  size_t               comp_size;

  memset(&xx, 0, sizeof(xx));
  xx.vault = vault;

  /*-------------------------------------------------------------------------
   * Establish transport connection with responder
   *-----------------------------------------------------------------------*/
  error = establish_responder_connection(&transport, ip_address, &xx.p_reader, &xx.p_writer);
  if (TRANSPORT_ERROR_NONE != error) {
    log_error(error, "Failed to establish connection with responder");
    goto exit;
  }

  /*-------------------------------------------------------------------------
   * Perform the secret xx
   * If successful, encrypt/decrypt keys will be established
   *-----------------------------------------------------------------------*/

  if (scripted_xx) {
    error = test_responder_handshake(&xx);
  } else {
    error = ockam_key_establish_responder_xx(&xx);
  }
  if (error) {
    log_error(error, "ockam_responder_handshake");
    goto exit;
  }

  /*-------------------------------------------------------------------------
   * Verify secure channel by sending and receiving a known message
   *-----------------------------------------------------------------------*/

  if (scripted_xx) {
    /* Convert string to hex bytes and encrypt */
    string_to_hex((uint8_t*) TEST_MSG_RESPONDER, test, &test_size);
    error = xx_encrypt(&xx, test, test_size, sendBuffer, sizeof(sendBuffer), &transmit_size);
    if (error != TRANSPORT_ERROR_NONE) {
      log_error(error, "responder_epilogue_make failed");
      goto exit;
    }
    /* Verify test message ciphertext */
    string_to_hex((uint8_t*) MSG_4_CIPHERTEXT, comp, &comp_size);
    if (0 != memcmp(comp, sendBuffer, transmit_size)) {
      error = kXXKeyAgreementTestFailed;
      log_error(error, "Msg 4 failed");
      goto exit;
    }
  } else {
    error = xx_encrypt(&xx, (uint8_t*) ACK, ACK_SIZE, sendBuffer, sizeof(sendBuffer), &transmit_size);
    if (error != TRANSPORT_ERROR_NONE) {
      log_error(error, "responder_epilogue_make failed");
      goto exit;
    }
  }

  /* Send test message */
  error = ockam_write(xx.p_writer, sendBuffer, transmit_size);
  if (error != TRANSPORT_ERROR_NONE) {
    log_error(error, "ockam_SendBlocking epilogue failed");
    goto exit;
  }

  /* Receive test message  */
  memset(recv_buffer, 0, sizeof(recv_buffer));
  error = ockam_read(xx.p_reader, recv_buffer, MAX_TRANSMIT_SIZE, &transmit_size);
  if (error != TRANSPORT_ERROR_NONE) {
    log_error(error, "ockam_ReceiveBlocking failed for msg 3");
    goto exit;
  }

  /* Decrypt test message */

  error = xx_decrypt(&xx, test, TEST_MSG_BYTE_SIZE, recv_buffer, transmit_size, &test_size);
  if (TRANSPORT_ERROR_NONE != error) {
    log_error(error, "ockam_ReceiveBlocking failed on msg 2");
    goto exit;
  }

  /* Verify test message */
  if (scripted_xx) {
    string_to_hex((uint8_t*) TEST_MSG_INITIATOR, test_initiator, NULL);
    if (0 != memcmp((void*) test, test_initiator, TEST_MSG_BYTE_SIZE)) {
      error = kXXKeyAgreementTestFailed;
      log_error(error, "Received bad test message");
      goto exit;
    }
  } else {
    if (0 != memcmp(OK, test, OK_SIZE)) {
      error = kXXKeyAgreementTestFailed;
      log_error(error, "Received bad test message");
      goto exit;
    }
  }
  error = OCKAM_ERROR_NONE;
exit:
  if (NULL != transport) ockam_transport_deinit(transport);
  printf("Test ended with error %0.4x\n", error);
  return error;
}
