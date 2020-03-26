/**
 ********************************************************************************************************
 * @file    xx_responder_test.c
 * @brief   Test program for the xx xx as per Noise XX 25519 AESGCM SHA256
 ********************************************************************************************************
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../../xx/xx_local.h"
#include "ockam/error.h"
#include "ockam/key_agreement.h"
#include "ockam/memory.h"
#include "ockam/syslog.h"
#include "ockam/transport.h"
#include "ockam/vault.h"
#include "xx_test.h"

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
 * @return [out] - kErrorNone on success
 ********************************************************************************************************
 */
OckamError OckamErrorXXTestResponderPrologue(KeyEstablishmentXX *xx) {
  OckamError status = kErrorNone;
  uint8_t key[KEY_SIZE];
  uint32_t key_bytes;

  // 1. Pick a static 25519 keypair for this xx and set it to s
  string_to_hex(RESPONDER_STATIC, key, &key_bytes);
  status = xx->vault->KeySetPrivate(xx->vault_ctx, kOckamVaultKeyStatic, key, KEY_SIZE);
  if (kErrorNone != status) {
    log_error(status, "failed to generate static keypair in initiator_step_1");
    goto exit_block;
  }

  status = xx->vault->KeyGetPublic(xx->vault_ctx, kOckamVaultKeyStatic, xx->s, KEY_SIZE);
  if (kErrorNone != status) {
    log_error(status, "failed to generate get static public key in initiator_step_1");
    goto exit_block;
  }

  // 2. Generate an ephemeral 25519 keypair for this xx and set it to e
  string_to_hex(RESPONDER_EPH, key, &key_bytes);
  status = xx->vault->KeySetPrivate(xx->vault_ctx, kOckamVaultKeyEphemeral, key, KEY_SIZE);
  if (kErrorNone != status) {
    log_error(status, "failed to generate static keypair in initiator_step_1");
    goto exit_block;
  }

  status = xx->vault->KeyGetPublic(xx->vault_ctx, kOckamVaultKeyEphemeral, xx->e, KEY_SIZE);
  if (kErrorNone != status) {
    log_error(status, "failed to generate get static public key in initiator_step_1");
    goto exit_block;
  }

  // Nonce to 0, k to empty
  xx->nonce = 0;
  memset(xx->k, 0, sizeof(xx->k));

  // Initialize h
  memset(&xx->h[0], 0, SHA256_SIZE);
  memcpy(&xx->h[0], PROTOCOL_NAME, PROTOCOL_NAME_SIZE);

  // Initialize ck
  memset(&xx->ck[0], 0, KEY_SIZE);
  memcpy(&xx->ck[0], PROTOCOL_NAME, PROTOCOL_NAME_SIZE);

  // h = SHA256(h || prologue), prologue is empty
  mix_hash(xx, NULL, 0);

exit_block:
  return status;
}

/**
 ********************************************************************************************************
 *                                          TestResponderHandshake()
 ********************************************************************************************************
 *
 * Summary: Test the xx process by starting with predefined static and ephemeral
 *keys (generated in the prologue) and verifying intermediate results against
 *test data along the way
 *
 * @param connection [in] - initialized transport connection
 * @param xx [in/out] - pointer to xx structure
 * @return [out] - kErrorNone on success
 ********************************************************************************************************
 */
OckamError TestResponderHandshake(const OckamVault *vault, OckamVaultCtx *vaultCtx, const OckamTransport *transport,
                                  OckamTransportCtx transportCtx, KeyEstablishmentXX *xx) {
  OckamError status = kErrorNone;
  uint8_t sendBuffer[MAX_TRANSMIT_SIZE];
  uint8_t recv_buffer[MAX_TRANSMIT_SIZE];
  uint16_t transmit_size = 0;
  uint16_t bytesReceived = 0;
  uint8_t compare[1024];
  uint32_t compare_bytes;

  /* Initialize the KeyEstablishmentXX struct */
  memset(xx, 0, sizeof(*xx));
  OckamKeyInitializeXX(xx, vault, vaultCtx, transport, transportCtx);

  /* Prologue initializes keys and xx parameters */
  status = OckamErrorXXTestResponderPrologue(xx);
  if (kErrorNone != status) {
    log_error(status, "test_xx_prologue failed");
    goto exit_block;
  }
  /* Msg 1 receive */
  status = xx->transport->Read(xx->transport_ctx, &recv_buffer[0], MAX_TRANSMIT_SIZE, &bytesReceived);
  if (status != kErrorNone) {
    log_error(status, "Read for msg 1 failed");
    goto exit_block;
  }

  /* Msg 1 process */
  status = XXResponderM1Process(xx, recv_buffer, bytesReceived);
  if (status != kErrorNone) {
    log_error(status, "XXResponderM1Process failed");
    goto exit_block;
  }

  /* Msg 2 make */
  status = XXResponderM2Make(xx, sendBuffer, sizeof(sendBuffer), &transmit_size);
  if (status != kErrorNone) {
    log_error(status, "XXResponderM2Make failed");
    goto exit_block;
  }

  /* Msg 2 verify */
  string_to_hex(MSG_2_CIPHERTEXT, compare, &compare_bytes);
  if (0 != memcmp(sendBuffer, compare, compare_bytes)) {
    log_error(status, "Test failed on msg 2\n");
    goto exit_block;
  }

  /* Msg 2 send */
  status = xx->transport->Write(xx->transport_ctx, sendBuffer, transmit_size);
  if (status != kErrorNone) {
    log_error(status, "responder_m2_send failed");
    goto exit_block;
  }

  /* Msg 3 receive */
  status = xx->transport->Read(xx->transport_ctx, recv_buffer, MAX_TRANSMIT_SIZE, &bytesReceived);
  if (status != kErrorNone) {
    log_error(status, "ockam_ReceiveBlocking failed for msg 3");
    goto exit_block;
  }

  /* Msg 3 process */
  status = XXResponderM3Process(xx, recv_buffer, bytesReceived);
  if (status != kErrorNone) {
    log_error(status, "responder_m3_process failed for msg 3");
    goto exit_block;
  }

  /* Epilogue */
  status = XXResponderEpilogue(xx);
  if (kErrorNone != status) {
    log_error(status, "Failed responder_epilogue");
    goto exit_block;
  }

exit_block:
  return status;
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
OckamError EstablishResponderConnection(int argc, char *argv[], const OckamTransport *transport,
                                        OckamTransportCtx *listenerCtx, OckamTransportCtx *connectionCtx) {
  OckamError status = kErrorNone;
  OckamInternetAddress listener_address;
  OckamTransportConfig tcpConfig = {kBlocking};

  // Get the IP address to listen on
  status = GetIpInfo(argc, argv, &listener_address);
  if (kErrorNone != status) {
    log_error(status, "failed to get address into");
    goto exit_block;
  }

  status = transport->Create(listenerCtx, &tcpConfig);
  if (kErrorNone != status) {
    log_error(status, "failed PosixTcpInitialize");
    goto exit_block;
  }

  // Wait for a connection
  status = transport->Listen(*listenerCtx, &listener_address, connectionCtx);
  if (kErrorNone != status) {
    log_error(status, "listen failed");
    goto exit_block;
  }

exit_block:
  return status;
}

/**
 ********************************************************************************************************
 *                                   main()
 ********************************************************************************************************
 *
 * @return - 0 on success
 */

extern const OckamTransport ockamPosixTcpTransport;

OckamError XXTestResponder(int argc, char *argv[], const OckamVault *vault, void *vaultCtx) {
  OckamError status = kErrorNone;
  const OckamTransport *transport = &ockamPosixTcpTransport;
  OckamTransportCtx listenerCtx = NULL;
  OckamTransportCtx connectionCtx = NULL;

  KeyEstablishmentXX xx;
  uint8_t sendBuffer[MAX_TRANSMIT_SIZE];
  uint8_t recv_buffer[MAX_TRANSMIT_SIZE];
  uint16_t transmit_size = 0;
  uint8_t test[16];
  uint32_t test_size;
  uint8_t test_initiator[TEST_MSG_BYTE_SIZE];
  uint8_t comp[2048];
  uint32_t comp_size;

  /*-------------------------------------------------------------------------
   * Establish transport connection with responder
   *-----------------------------------------------------------------------*/

  status = EstablishResponderConnection(argc, argv, transport, &listenerCtx, &connectionCtx);
  if (kErrorNone != status) {
    log_error(status, "Failed to establish connection with responder");
    goto exit_block;
  }

  printf("Connection established\n");
  /*-------------------------------------------------------------------------
   * Perform the secret xx
   * If successful, encrypt/decrypt keys will be established
   *-----------------------------------------------------------------------*/

  memset(&xx, 0, sizeof(xx));
  status = TestResponderHandshake(vault, vaultCtx, transport, connectionCtx, &xx);
  if (kErrorNone != status) {
    log_error(status, "ockam_responder_xx failed");
    goto exit_block;
  }

  /*-------------------------------------------------------------------------
   * Verify secure channel by sending and receiving a known message
   *-----------------------------------------------------------------------*/

  /* Convert string to hex bytes and encrypt */
  string_to_hex(TEST_MSG_RESPONDER, test, &test_size);
  status = XXEncrypt(&xx, test, test_size, sendBuffer, sizeof(sendBuffer), &transmit_size);
  if (status != kErrorNone) {
    log_error(status, "responder_epilogue_make failed");
    goto exit_block;
  }
  /* Verify test message ciphertext */
  string_to_hex(MSG_4_CIPHERTEXT, comp, &comp_size);
  if (0 != memcmp(comp, sendBuffer, transmit_size)) {
    status = kXXKeyAgreementTestFailed;
    log_error(status, "Msg 4 failed");
    goto exit_block;
  }

  /* Send test message */
  status = transport->Write(connectionCtx, sendBuffer, transmit_size);
  if (status != kErrorNone) {
    log_error(status, "ockam_SendBlocking epilogue failed");
    goto exit_block;
  }

  /* Receive test message  */
  status = transport->Read(connectionCtx, recv_buffer, MAX_TRANSMIT_SIZE, &transmit_size);
  if (status != kErrorNone) {
    log_error(status, "ockam_ReceiveBlocking failed for msg 3");
    goto exit_block;
  }

  /* Decrypt test message */

  status = XXDecrypt(&xx, test, TEST_MSG_BYTE_SIZE, recv_buffer, transmit_size, &test_size);
  if (kErrorNone != status) {
    log_error(status, "ockam_ReceiveBlocking failed on msg 2");
    goto exit_block;
  }

  /* Verify test message */

  string_to_hex(TEST_MSG_INITIATOR, test_initiator, NULL);
  if (0 != memcmp((void *)test, test_initiator, TEST_MSG_BYTE_SIZE)) {
    status = kXXKeyAgreementTestFailed;
    log_error(status, "Received bad test message");
    goto exit_block;
  }

exit_block:
  if (NULL != connectionCtx) transport->Destroy(connectionCtx);
  if (NULL != listenerCtx) transport->Destroy(listenerCtx);
  printf("Test ended with status %0.4x\n", status);
  return status;
}
